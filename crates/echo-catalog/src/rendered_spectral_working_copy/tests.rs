use std::path::Path;

use echo_domain::{AdjustmentGraph, ContentHash};

use super::*;
use crate::{
    AssetRegistrationInput, RegisterAsset, open_catalog, record_adjustment_graph, register_asset,
};

#[test]
fn working_copy_freezes_render_evidence_and_becomes_unavailable_after_upstream_change() {
    let root = std::env::temp_dir().join(format!(
        "echo-rendered-spectral-working-copy-{}",
        uuid::Uuid::now_v7()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (asset_id, first_revision) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let asset_id = match register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([5; 32]),
                    path: Path::new("/source.wav"),
                    size_bytes: 20,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            let first_revision = record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::identity(1_000).expect("identity graph validates"),
                2,
            )?;
            Ok((asset_id, first_revision))
        })
        .expect("source and current graph record");
    let created = catalog
        .with_transaction(|transaction| {
            create_rendered_spectral_working_copy(
                transaction,
                CreateRenderedSpectralWorkingCopy {
                    asset_id,
                    parent_adjustment_revision_id: first_revision.revision_id,
                    parent_render_content_hash: ContentHash::new([9; 32]),
                    created_at_millis: 4,
                },
            )
        })
        .expect("working copy creates");
    assert_eq!(
        created.parent_adjustment_revision_id,
        first_revision.revision_id
    );
    assert_eq!(
        created.parent_render_content_hash,
        ContentHash::new([9; 32])
    );
    assert!(created.enabled);
    assert_eq!(
        created.availability,
        RenderedSpectralWorkingCopyAvailability::Available
    );

    let committed = catalog
        .with_transaction(|transaction| {
            commit_rendered_spectral_erase(
                transaction,
                CommitRenderedSpectralErase {
                    asset_id,
                    working_copy_id: created.id,
                    rendered_content_hash: ContentHash::new([10; 32]),
                    start_millis: 100,
                    end_millis: 220,
                    low_hertz: 240,
                    high_hertz: 1_800,
                    attenuation_centibels: 9_600,
                    time_feather_millis: 24,
                    frequency_feather_hertz: 60,
                },
            )
        })
        .expect("erase commit updates the shared working copy");
    assert_eq!(
        committed.parent_render_content_hash,
        ContentHash::new([9; 32])
    );
    assert_eq!(
        committed.working_render_content_hash,
        ContentHash::new([10; 32])
    );
    assert_eq!(committed.operation_count, 1);
    assert!(committed.tile_manifest_json.contains("\"erase\""));

    catalog
        .with_transaction(|transaction| {
            set_rendered_spectral_working_copy_enabled(transaction, asset_id, created.id, false)
        })
        .expect("whole copy can bypass");
    let disabled = catalog
        .with_transaction(|transaction| {
            list_rendered_spectral_working_copies(transaction, asset_id)
        })
        .expect("working copies list");
    assert!(!disabled[0].enabled);
    assert_eq!(
        disabled[0].availability,
        RenderedSpectralWorkingCopyAvailability::Available
    );

    catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::new(
                    1_000,
                    10,
                    990,
                    0,
                    0,
                    echo_domain::AdjustmentEffects::default(),
                )
                .expect("changed graph validates"),
                5,
            )
        })
        .expect("upstream graph changes");
    let unavailable = catalog
        .with_transaction(|transaction| {
            list_rendered_spectral_working_copies(transaction, asset_id)
        })
        .expect("working copies list after change");
    assert_eq!(unavailable.len(), 1);
    assert_eq!(
        unavailable[0].availability,
        RenderedSpectralWorkingCopyAvailability::UpstreamChanged
    );
    let error = catalog
        .with_transaction(|transaction| {
            create_rendered_spectral_working_copy(
                transaction,
                CreateRenderedSpectralWorkingCopy {
                    asset_id,
                    parent_adjustment_revision_id: first_revision.revision_id,
                    parent_render_content_hash: ContentHash::new([9; 32]),
                    created_at_millis: 6,
                },
            )
        })
        .expect_err("stale render cannot seed a new copy");
    assert_eq!(error.kind, CatalogErrorKind::Constraint);

    catalog
        .with_transaction(|transaction| {
            remove_rendered_spectral_working_copy(transaction, asset_id, created.id)
        })
        .expect("whole copy removes");
    assert!(
        catalog
            .with_transaction(|transaction| list_rendered_spectral_working_copies(
                transaction,
                asset_id
            ))
            .expect("working copies list after removal")
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn working_copy_rejects_a_render_that_is_already_stale() {
    let root = std::env::temp_dir().join(format!(
        "echo-rendered-spectral-working-copy-stale-{}",
        uuid::Uuid::now_v7()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let asset_id = register(transaction, [4; 32], "/first.wav")?;
            record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::identity(1_000).expect("identity graph validates"),
                2,
            )?;
            Ok(asset_id)
        })
        .expect("asset and current graph record");
    let error = catalog
        .with_transaction(|transaction| {
            create_rendered_spectral_working_copy(
                transaction,
                CreateRenderedSpectralWorkingCopy {
                    asset_id,
                    parent_adjustment_revision_id: 0,
                    parent_render_content_hash: ContentHash::new([8; 32]),
                    created_at_millis: 3,
                },
            )
        })
        .expect_err("stale render is rejected");
    assert_eq!(error.kind, CatalogErrorKind::Constraint);
    assert!(
        catalog
            .with_transaction(|transaction| list_rendered_spectral_working_copies(
                transaction,
                asset_id
            ))
            .expect("asset lists")
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(root);
}

fn register(
    transaction: &rusqlite::Transaction<'_>,
    content_hash: [u8; 32],
    path: &str,
) -> Result<echo_domain::AssetId, CatalogError> {
    Ok(
        match register_asset(
            transaction,
            &AssetRegistrationInput {
                content_hash: ContentHash::new(content_hash),
                path: Path::new(path),
                size_bytes: 20,
                codec: Some("pcm"),
                duration_millis: Some(1_000),
                recorded_at_millis: None,
                imported_at_millis: 1,
            },
        )? {
            RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
        },
    )
}
