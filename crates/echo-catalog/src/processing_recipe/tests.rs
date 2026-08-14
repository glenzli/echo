use std::path::Path;

use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, AdjustmentPatch, AssetId, CompressorSettings, ContentHash,
    FadeCurve, FadeCurves, ProcessingComponent, ProcessingMergeMode, SpaceMode, SpaceSettings,
};
use rusqlite::Transaction;

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
#[allow(clippy::too_many_lines)] // One legacy fixture proves failure ordering and no writes.
fn legacy_recipe_with_missing_ir_fails_each_target_without_writing_adjustments() {
    let root = fixture_root("recipe-missing-ir");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok(register(transaction, 0x92, "/voices/recipe-ir.wav", 10_000))
        })
        .expect("asset registers");
    let selection = echo_domain::ImpulseResponseSelection {
        import_id: uuid::Uuid::now_v7(),
        source_hash: ContentHash::new([0xa2; 32]),
        prepared_hash: ContentHash::new([0xb2; 32]),
    };
    let graph = AdjustmentGraph::new(
        10_000,
        0,
        10_000,
        0,
        0,
        AdjustmentEffects::new(FadeCurves::linear(), 0, 0).with_space(SpaceSettings {
            mode: SpaceMode::Convolution,
            impulse_response: Some(selection),
            ..SpaceSettings::default()
        }),
    )
    .expect("recipe source graph validates");
    let patch = AdjustmentPatch::from_graph(graph, &[ProcessingComponent::Space])
        .expect("space patch validates");
    assert!(
        catalog
            .with_transaction(|transaction| {
                create_processing_recipe(
                    transaction,
                    CreateProcessingRecipe {
                        name: "Missing IR",
                        patch: &patch,
                    },
                    9,
                )
            })
            .is_err()
    );
    let valid_recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "Valid source",
                    patch: &patch_with_low_cut_and_dynamics(120, -1_800),
                },
                9,
            )
        })
        .expect("recipe without convolution IR creates");
    assert!(
        catalog
            .with_transaction(|transaction| {
                append_processing_recipe_revision(transaction, valid_recipe.id, &patch, 10)
            })
            .is_err()
    );
    let recipe_id = ProcessingRecipeId::new();
    let revision_id = ProcessingRecipeRevisionId::new();
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            transaction.execute(
                "INSERT INTO processing_recipes
                 (id, name, created_at_millis, updated_at_millis)
                 VALUES (?1, 'Legacy missing IR', 10, 10)",
                [recipe_id.to_string()],
            )?;
            transaction.execute(
                "INSERT INTO processing_recipe_revisions
                 (id, recipe_id, revision_number, patch_json, created_at_millis)
                 VALUES (?1, ?2, 1, ?3, 10)",
                rusqlite::params![
                    revision_id.to_string(),
                    recipe_id.to_string(),
                    serde_json::to_string(&patch).expect("patch encodes"),
                ],
            )?;
            Ok(())
        })
        .expect("legacy malformed recipe fixture writes");
    let receipt = catalog
        .with_transaction(|transaction| {
            apply_processing_recipe(
                transaction,
                recipe_id,
                &[asset_id],
                ProcessingMergeMode::Merge,
                20,
            )
        })
        .expect("application records failure receipt");
    assert_eq!(
        receipt.targets[0].outcome,
        ProcessingRecipeTargetOutcome::Failed
    );
    assert!(
        receipt.targets[0]
            .resulting_adjustment_revision_id
            .is_none()
    );
    assert!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, asset_id))
            .expect("target reads")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(root);
}

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
    let history = catalog
        .with_transaction(list_processing_recipe_application_history)
        .expect("processing history lists");
    assert_eq!(
        history
            .iter()
            .map(|entry| entry.batch_id)
            .collect::<Vec<_>>(),
        vec![repeated.batch_id, receipt.batch_id]
    );
    assert_eq!(history[0].target_count, 2);
    assert_eq!(history[0].updated_count, 0);
    assert_eq!(history[0].unchanged_count, 2);
    assert_eq!(history[0].failed_count, 0);
    assert_eq!(history[1].target_count, 3);
    assert_eq!(history[1].updated_count, 2);
    assert_eq!(history[1].unchanged_count, 0);
    assert_eq!(history[1].failed_count, 1);
    assert!(history.iter().all(|entry| entry.revert.is_none()));
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

#[test]
fn patch_capture_uses_latest_asset_graph_or_duration_identity() {
    let root = fixture_root("recipe-patch-capture");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok(register(transaction, 10, "/voices/capture.wav", 6_000))
        })
        .expect("asset registers");
    let initial = catalog
        .with_transaction(|transaction| {
            processing_recipe_patch_from_asset(
                transaction,
                asset_id,
                &[ProcessingComponent::Master],
            )
        })
        .expect("identity patch captures");
    assert_eq!(initial.components(), &[ProcessingComponent::Master]);

    let authored = configured_graph(6_000, 500, 5_500, -100, 140, -2_000);
    catalog
        .with_transaction(|transaction| record_adjustment_graph(transaction, asset_id, authored, 5))
        .expect("authored graph writes");
    let captured = catalog
        .with_transaction(|transaction| {
            processing_recipe_patch_from_asset(
                transaction,
                asset_id,
                &[ProcessingComponent::LowCut],
            )
        })
        .expect("authored patch captures");
    let applied = captured
        .apply_to(
            AdjustmentGraph::identity(6_000).expect("identity validates"),
            ProcessingMergeMode::Merge,
        )
        .expect("captured patch applies");
    assert_eq!(applied.low_cut_hertz(), 140);
    assert!(
        catalog
            .with_transaction(|transaction| {
                processing_recipe_patch_from_asset(
                    transaction,
                    AssetId::new(),
                    &[ProcessingComponent::LowCut],
                )
            })
            .is_err()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // Lifecycle contract keeps history assertions together.
fn rename_and_archive_preserve_identity_revisions_and_application_history() {
    let root = fixture_root("recipe-management");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok(register(transaction, 11, "/voices/managed.wav", 8_000))
        })
        .expect("asset registers");
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
    let application = catalog
        .with_transaction(|transaction| {
            apply_processing_recipe(
                transaction,
                recipe.id,
                &[asset_id],
                ProcessingMergeMode::Merge,
                20,
            )
        })
        .expect("recipe applies");

    let renamed = catalog
        .with_transaction(|transaction| {
            rename_processing_recipe(transaction, recipe.id, "Field dialogue", 30)
        })
        .expect("recipe renames");
    assert_eq!(renamed.id, recipe.id);
    assert_eq!(renamed.current_revision, recipe.current_revision);
    assert_eq!(renamed.name, "Field dialogue");
    assert!(
        catalog
            .with_transaction(|transaction| archive_processing_recipe(transaction, recipe.id, 40))
            .expect("recipe archives")
    );
    assert!(
        !catalog
            .with_transaction(|transaction| archive_processing_recipe(transaction, recipe.id, 50))
            .expect("second archive is idempotent")
    );

    assert!(
        catalog
            .with_transaction(list_processing_recipes)
            .expect("recipes list")
            .is_empty()
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| processing_recipe(transaction, recipe.id))
            .expect("recipe query succeeds"),
        None
    );
    assert!(
        catalog
            .with_transaction(|transaction| {
                append_processing_recipe_revision(transaction, recipe.id, &patch, 60)
            })
            .is_err()
    );
    assert!(
        catalog
            .with_transaction(|transaction| {
                rename_processing_recipe(transaction, recipe.id, "Archived rename", 60)
            })
            .is_err()
    );
    assert!(
        catalog
            .with_transaction(|transaction| {
                apply_processing_recipe(
                    transaction,
                    recipe.id,
                    &[asset_id],
                    ProcessingMergeMode::Merge,
                    60,
                )
            })
            .is_err()
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                processing_recipe_application_receipt(transaction, application.batch_id)
            })
            .expect("historical application reads")
            .expect("historical application remains"),
        application
    );
    let history = catalog
        .with_transaction(list_processing_recipe_application_history)
        .expect("archived recipe history lists");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].batch_id, application.batch_id);
    assert_eq!(history[0].recipe_id, recipe.id);
    assert_eq!(history[0].recipe_name, "Field dialogue");
    assert_eq!(
        history[0].recipe_revision_id,
        recipe.current_revision.revision_id()
    );
    assert_eq!(history[0].recipe_revision_number, 1);
    assert_eq!(history[0].merge_mode, ProcessingMergeMode::Merge);
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
    assert_eq!(revision_count, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One receipt contract covers every stable target outcome.
fn batch_revert_restores_safe_targets_once_and_preserves_later_edits() {
    let root = fixture_root("recipe-revert");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (restore_previous, restore_identity, unchanged, conflict) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                register(transaction, 21, "/voices/previous.wav", 10_000),
                register(transaction, 22, "/voices/identity.wav", 8_000),
                register(transaction, 23, "/voices/unchanged.wav", 9_000),
                register(transaction, 24, "/voices/conflict.wav", 7_000),
            ))
        })
        .expect("assets register");
    let previous_graph = configured_graph(10_000, 1_000, 9_000, -250, 0, -1_800);
    let previous_revision = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, restore_previous, previous_graph.clone(), 5)
        })
        .expect("previous adjustment writes");
    let matching_graph = configured_graph(9_000, 500, 8_500, 100, 100, -2_100);
    catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, unchanged, matching_graph, 5)
        })
        .expect("matching adjustment writes");

    let patch = patch_with_low_cut_and_dynamics(100, -2_100);
    let recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "Safe revert fixture",
                    patch: &patch,
                },
                10,
            )
        })
        .expect("recipe creates");
    let missing = AssetId::new();
    let application = catalog
        .with_transaction(|transaction| {
            apply_processing_recipe(
                transaction,
                recipe.id,
                &[
                    restore_previous,
                    restore_identity,
                    unchanged,
                    missing,
                    conflict,
                ],
                ProcessingMergeMode::Merge,
                20,
            )
        })
        .expect("recipe applies");
    assert_eq!(
        application
            .targets
            .iter()
            .map(|target| target.outcome)
            .collect::<Vec<_>>(),
        vec![
            ProcessingRecipeTargetOutcome::Updated,
            ProcessingRecipeTargetOutcome::Updated,
            ProcessingRecipeTargetOutcome::Unchanged,
            ProcessingRecipeTargetOutcome::Failed,
            ProcessingRecipeTargetOutcome::Updated,
        ]
    );
    let later_graph = configured_graph(7_000, 0, 7_000, 300, 80, -1_600);
    let later_revision = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, conflict, later_graph, 25)
        })
        .expect("later edit writes");

    let receipt = catalog
        .with_transaction(|transaction| {
            revert_processing_recipe_application(transaction, application.batch_id, 30)
        })
        .expect("application reverts");
    assert_eq!(
        receipt
            .targets
            .iter()
            .map(|target| target.outcome)
            .collect::<Vec<_>>(),
        vec![
            ProcessingRecipeRevertTargetOutcome::Restored,
            ProcessingRecipeRevertTargetOutcome::Restored,
            ProcessingRecipeRevertTargetOutcome::Unchanged,
            ProcessingRecipeRevertTargetOutcome::Unchanged,
            ProcessingRecipeRevertTargetOutcome::Conflict,
        ]
    );
    let restored_revision = catalog
        .with_transaction(|transaction| latest_adjustment_graph(transaction, restore_previous))
        .expect("restored graph reads")
        .expect("restored graph exists");
    assert_eq!(restored_revision.graph, previous_graph);
    assert_ne!(restored_revision.revision_id, previous_revision.revision_id);
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, restore_identity))
            .expect("identity graph reads")
            .expect("identity graph exists")
            .graph,
        AdjustmentGraph::identity(8_000).expect("identity validates")
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, conflict))
            .expect("conflict graph reads")
            .expect("conflict graph exists")
            .revision_id,
        later_revision.revision_id
    );
    let adjustment_count = catalog
        .with_transaction(|transaction| -> Result<i64, CatalogError> {
            transaction
                .query_row(
                    "SELECT COUNT(*) FROM asset_adjustment_revisions",
                    [],
                    |row| row.get(0),
                )
                .map_err(CatalogError::from)
        })
        .expect("adjustment count reads");
    let repeated = catalog
        .with_transaction(|transaction| {
            revert_processing_recipe_application(transaction, application.batch_id, 40)
        })
        .expect("second revert reads first receipt");
    assert_eq!(repeated, receipt);
    assert_eq!(
        catalog
            .with_transaction(|transaction| -> Result<i64, CatalogError> {
                transaction
                    .query_row(
                        "SELECT COUNT(*) FROM asset_adjustment_revisions",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(CatalogError::from)
            })
            .expect("adjustment count rereads"),
        adjustment_count
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                processing_recipe_application_revert_receipt(transaction, application.batch_id)
            })
            .expect("revert receipt reads")
            .expect("revert receipt exists"),
        receipt
    );
    let history = catalog
        .with_transaction(list_processing_recipe_application_history)
        .expect("reverted history lists");
    assert_eq!(history.len(), 1);
    let entry = &history[0];
    assert_eq!(entry.batch_id, application.batch_id);
    assert_eq!(entry.target_count, 5);
    assert_eq!(entry.updated_count, 3);
    assert_eq!(entry.unchanged_count, 1);
    assert_eq!(entry.failed_count, 1);
    let revert = entry.revert.as_ref().expect("revert summary exists");
    assert_eq!(revert.revert_id, receipt.revert_id);
    assert_eq!(revert.restored_count, 2);
    assert_eq!(revert.unchanged_count, 2);
    assert_eq!(revert.conflict_count, 1);
    assert_eq!(revert.failed_count, 0);
    assert_eq!(revert.created_at_millis, 30);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn processing_history_is_bounded_to_the_hundred_newest_batches() {
    let root = fixture_root("recipe-history-bound");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok(register(transaction, 28, "/voices/history.wav", 8_000))
        })
        .expect("asset registers");
    let patch = patch_with_low_cut_and_dynamics(100, -2_100);
    let recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "History bound",
                    patch: &patch,
                },
                10,
            )
        })
        .expect("recipe creates");
    let batch_ids = catalog
        .with_transaction(|transaction| -> Result<Vec<_>, CatalogError> {
            (0_i64..101)
                .map(|offset| {
                    apply_processing_recipe(
                        transaction,
                        recipe.id,
                        &[asset_id],
                        ProcessingMergeMode::Merge,
                        20 + offset,
                    )
                    .map(|receipt| receipt.batch_id)
                })
                .collect()
        })
        .expect("application history writes");
    let history = catalog
        .with_transaction(list_processing_recipe_application_history)
        .expect("bounded history lists");
    assert_eq!(history.len(), 100);
    assert_eq!(history[0].batch_id, batch_ids[100]);
    assert_eq!(history[99].batch_id, batch_ids[1]);
    assert!(history.iter().all(|entry| entry.batch_id != batch_ids[0]));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // The injected storage fault proves transactional rollback.
fn revert_database_failure_rolls_back_receipt_and_restored_adjustments() {
    let root = fixture_root("recipe-revert-rollback");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (first, second) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                register(transaction, 31, "/voices/rollback-first.wav", 8_000),
                register(transaction, 32, "/voices/rollback-second.wav", 8_000),
            ))
        })
        .expect("assets register");
    let patch = patch_with_low_cut_and_dynamics(100, -2_100);
    let recipe = catalog
        .with_transaction(|transaction| {
            create_processing_recipe(
                transaction,
                CreateProcessingRecipe {
                    name: "Rollback fixture",
                    patch: &patch,
                },
                10,
            )
        })
        .expect("recipe creates");
    let application = catalog
        .with_transaction(|transaction| {
            apply_processing_recipe(
                transaction,
                recipe.id,
                &[first, second],
                ProcessingMergeMode::Merge,
                20,
            )
        })
        .expect("recipe applies");
    let first_result = application.targets[0]
        .resulting_adjustment_revision_id
        .expect("first result exists");
    let adjustment_count = catalog
        .with_transaction(|transaction| -> Result<i64, CatalogError> {
            transaction
                .query_row(
                    "SELECT COUNT(*) FROM asset_adjustment_revisions",
                    [],
                    |row| row.get(0),
                )
                .map_err(CatalogError::from)
        })
        .expect("adjustment count reads");
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            transaction.execute_batch(&format!(
                "CREATE TRIGGER reject_second_revert_target \
                 BEFORE INSERT ON processing_recipe_application_revert_targets \
                 WHEN NEW.asset_id = '{second}' BEGIN \
                 SELECT RAISE(ABORT, 'injected revert receipt failure'); END;"
            ))?;
            Ok(())
        })
        .expect("failure trigger installs");

    assert!(
        catalog
            .with_transaction(|transaction| {
                revert_processing_recipe_application(transaction, application.batch_id, 30)
            })
            .is_err()
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                processing_recipe_application_revert_receipt(transaction, application.batch_id)
            })
            .expect("revert receipt query succeeds"),
        None
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| -> Result<i64, CatalogError> {
                transaction
                    .query_row(
                        "SELECT COUNT(*) FROM asset_adjustment_revisions",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(CatalogError::from)
            })
            .expect("adjustment count rereads"),
        adjustment_count
    );
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, first))
            .expect("first graph reads")
            .expect("first graph exists")
            .revision_id,
        first_result
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

fn configured_graph(
    duration_millis: u64,
    trim_start_millis: u64,
    trim_end_millis: u64,
    gain_centibels: i16,
    low_cut_hertz: u16,
    compressor_threshold_centibels: i16,
) -> AdjustmentGraph {
    AdjustmentGraph::new(
        duration_millis,
        trim_start_millis,
        trim_end_millis,
        0,
        0,
        AdjustmentEffects::new(FadeCurves::linear(), gain_centibels, low_cut_hertz)
            .with_compressor(CompressorSettings {
                enabled: true,
                threshold_centibels: compressor_threshold_centibels,
                ratio_tenths: 35,
                attack_millis: 12,
                release_millis: 160,
                makeup_centibels: 150,
            }),
    )
    .expect("configured graph validates")
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
