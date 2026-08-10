use std::path::Path;

use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, AdjustmentPatch, AssetId, CompressorSettings, ContentHash,
    FadeCurve, FadeCurves, ProcessingComponent, ProcessingMergeMode,
};
use rusqlite::Transaction;

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn named_recipe_keeps_stable_identity_across_immutable_revisions() {
    let root = fixture_root("recipe-revisions");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let first_patch = patch_with_low_cut_and_dynamics(120, -1_800);
    let recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "Clear field voice",
                    patch: &first_patch,
                },
                10,
            )
        })
        .expect("recipe creates");
    assert_eq!(recipe.current_revision.sequence(), 1);
    assert_eq!(recipe.current_revision.patch(), &first_patch);

    let second_patch = patch_with_low_cut_and_dynamics(90, -2_400);
    let second_revision = catalog
        .with_transaction(|transaction| {
            append_processing_recipe_revision(transaction, recipe.id, &second_patch, 20)
        })
        .expect("revision appends");
    assert_eq!(second_revision.recipe_id(), recipe.id);
    assert_eq!(second_revision.sequence(), 2);
    assert_ne!(
        second_revision.revision_id(),
        recipe.current_revision.revision_id()
    );

    let recipes = catalog
        .with_transaction(list_processing_recipes)
        .expect("recipes list");
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].id, recipe.id);
    assert_eq!(recipes[0].name, "Clear field voice");
    assert_eq!(recipes[0].current_revision, second_revision);
    let revision_count = catalog
        .with_transaction(|transaction| -> Result<i64, CatalogError> {
            transaction
                .query_row(
                    "SELECT COUNT(*) FROM processing_recipe_revisions WHERE recipe_id = ?1",
                    [recipe.id.to_string()],
                    |row| row.get(0),
                )
                .map_err(CatalogError::from)
        })
        .expect("revision count reads");
    assert_eq!(revision_count, 2);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One batch contract verifies materialization and its receipt.
fn batch_application_materializes_local_revisions_and_persists_partial_receipt() {
    let root = fixture_root("recipe-batch");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (first, second) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                register(transaction, 1, "/voices/first.wav", 10_000),
                register(transaction, 2, "/voices/second.wav", 8_000),
            ))
        })
        .expect("assets register");
    let authored = AdjustmentGraph::new(
        10_000,
        1_000,
        9_000,
        300,
        450,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
            -250,
            0,
        ),
    )
    .expect("target graph validates");
    let original_revision = catalog
        .with_transaction(|transaction| record_adjustment_graph(transaction, first, authored, 5))
        .expect("target revision writes");

    let patch = patch_with_low_cut_and_dynamics(100, -2_100);
    let recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "Dialogue cleanup",
                    patch: &patch,
                },
                10,
            )
        })
        .expect("recipe creates");
    let missing = AssetId::new();
    let receipt = catalog
        .with_transaction(|transaction| {
            apply_processing_recipe(
                transaction,
                recipe.id,
                &[first, second, missing, first],
                ProcessingMergeMode::Merge,
                20,
            )
        })
        .expect("batch applies");
    assert_eq!(receipt.targets.len(), 3);
    assert_eq!(
        receipt.targets[0].outcome,
        ProcessingRecipeTargetOutcome::Updated
    );
    assert_eq!(
        receipt.targets[0].previous_adjustment_revision_id,
        Some(original_revision.revision_id)
    );
    assert_eq!(
        receipt.targets[1].outcome,
        ProcessingRecipeTargetOutcome::Updated
    );
    assert_eq!(
        receipt.targets[2].outcome,
        ProcessingRecipeTargetOutcome::Failed
    );
    assert!(receipt.targets[2].failure_reason.is_some());

    let first_graph = catalog
        .with_transaction(|transaction| latest_adjustment_graph(transaction, first))
        .expect("first graph reads")
        .expect("first graph exists")
        .graph;
    assert_eq!(first_graph.trim_start_millis(), 1_000);
    assert_eq!(first_graph.trim_end_millis(), 9_000);
    assert_eq!(first_graph.fade_in_millis(), 300);
    assert_eq!(first_graph.fade_out_millis(), 450);
    assert_eq!(first_graph.gain_centibels(), -250);
    assert_eq!(first_graph.low_cut_hertz(), 100);
    assert_eq!(first_graph.compressor().threshold_centibels, -2_100);

    let persisted = catalog
        .with_transaction(|transaction| {
            processing_recipe_application_receipt(transaction, receipt.batch_id)
        })
        .expect("receipt reads")
        .expect("receipt exists");
    assert_eq!(persisted, receipt);

    let repeated = catalog
        .with_transaction(|transaction| {
            apply_processing_recipe(
                transaction,
                recipe.id,
                &[first, second],
                ProcessingMergeMode::Merge,
                30,
            )
        })
        .expect("batch repeats");
    assert!(
        repeated
            .targets
            .iter()
            .all(|target| target.outcome == ProcessingRecipeTargetOutcome::Unchanged)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recipe_names_and_target_batches_are_bounded() {
    let root = fixture_root("recipe-validation");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let patch = patch_with_low_cut_and_dynamics(80, -1_800);
    for invalid in ["", " padded", "padded ", "line\nbreak"] {
        assert!(
            catalog
                .with_transaction(|transaction| {
                    create_processing_recipe(
                        transaction,
                        CreateProcessingRecipe {
                            name: invalid,
                            patch: &patch,
                        },
                        1,
                    )
                })
                .is_err()
        );
    }
    let recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "Voice cleanup",
                    patch: &patch,
                },
                2,
            )
        })
        .expect("recipe creates");
    assert!(
        catalog
            .with_transaction(|transaction| {
                create_processing_recipe(
                    transaction,
                    CreateProcessingRecipe {
                        name: "voice cleanup",
                        patch: &patch,
                    },
                    3,
                )
            })
            .is_err()
    );
    assert!(
        catalog
            .with_transaction(|transaction| {
                apply_processing_recipe(transaction, recipe.id, &[], ProcessingMergeMode::Merge, 4)
            })
            .is_err()
    );
    let _ = std::fs::remove_dir_all(root);
}

fn patch_with_low_cut_and_dynamics(
    low_cut_hertz: u16,
    threshold_centibels: i16,
) -> AdjustmentPatch {
    let source = AdjustmentGraph::new(
        12_000,
        0,
        12_000,
        0,
        0,
        AdjustmentEffects::new(FadeCurves::linear(), 0, low_cut_hertz).with_compressor(
            CompressorSettings {
                enabled: true,
                threshold_centibels,
                ratio_tenths: 35,
                attack_millis: 12,
                release_millis: 160,
                makeup_centibels: 150,
            },
        ),
    )
    .expect("source graph validates");
    AdjustmentPatch::from_graph(
        source,
        &[ProcessingComponent::LowCut, ProcessingComponent::Dynamics],
    )
    .expect("patch validates")
}

fn register(transaction: &Transaction<'_>, byte: u8, path: &str, duration_millis: u64) -> AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path: Path::new(path),
            size_bytes: 100,
            codec: Some("pcm"),
            duration_millis: Some(duration_millis),
            recorded_at_millis: None,
            imported_at_millis: i64::from(byte),
        },
    )
    .expect("asset registers")
    {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "echo-{name}-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ))
}
