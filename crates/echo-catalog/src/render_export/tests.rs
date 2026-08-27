use std::path::Path;

use echo_domain::{AdjustmentEffects, AdjustmentGraph, ContentHash, FadeCurves};

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, record_adjustment_graph};

#[test]
fn publication_round_trips_and_rejects_stale_adjustment() {
    let root = std::env::temp_dir().join(format!("echo-render-export-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| {
            let registered = crate::register_asset(
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
            )?;
            Ok::<_, CatalogError>(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            })
        })
        .expect("asset registers");
    let revision = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(
                transaction,
                asset.id,
                AdjustmentGraph::new(
                    1_000,
                    100,
                    900,
                    0,
                    0,
                    AdjustmentEffects::new(FadeCurves::default(), 0, 0),
                )
                .expect("graph validates"),
                2,
            )
        })
        .expect("adjustment records");
    let evidence = RecordRenderExport {
        asset_id: asset.id,
        adjustment_revision_id: revision.revision_id,
        output_path: PathBuf::from("/exports/voice.wav"),
        format: RenderExportFormat::WavPcm24,
        sample_rate: 48_000,
        channel_count: 2,
        bit_depth: 24,
        frame_count: 38_400,
        content_hash: ContentHash::new([9; 32]),
        size_bytes: 230_444,
        integrated_lufs: -18.2,
        true_peak_dbtp: -1.1,
        created_at_millis: 3,
    };
    let record = catalog
        .with_transaction(|transaction| record_render_export(transaction, &evidence))
        .expect("publication records");
    let records = catalog
        .with_transaction(|transaction| list_render_exports(transaction, asset.id))
        .expect("publications list");
    assert_eq!(records, vec![record.clone()]);
    let repeated = catalog
        .with_transaction(|transaction| record_render_export(transaction, &evidence))
        .expect("repeated publication is idempotent");
    assert_eq!(repeated.id, record.id);

    for (format, bit_depth, suffix) in [
        (RenderExportFormat::WavPcm16, 16, "16.wav"),
        (RenderExportFormat::Flac24, 24, "24.flac"),
    ] {
        let mut variant = evidence.clone();
        variant.output_path = PathBuf::from(format!("/exports/voice-{suffix}"));
        variant.format = format;
        variant.bit_depth = bit_depth;
        catalog
            .with_transaction(|transaction| record_render_export(transaction, &variant))
            .expect("additional delivery format records");
    }

    catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(
                transaction,
                asset.id,
                AdjustmentGraph::new(
                    1_000,
                    200,
                    900,
                    0,
                    0,
                    AdjustmentEffects::new(FadeCurves::default(), 0, 0),
                )
                .expect("second graph validates"),
                4,
            )
        })
        .expect("second adjustment records");
    let error = catalog
        .with_transaction(|transaction| record_render_export(transaction, &evidence))
        .expect_err("stale revision is rejected");
    assert_eq!(error.kind, CatalogErrorKind::Constraint);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rendered_working_copy_export_snapshots_mutable_repair_provenance() {
    let root = std::env::temp_dir().join(format!(
        "echo-rendered-working-copy-export-{}",
        std::process::id()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| {
            let registered = crate::register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([4; 32]),
                    path: Path::new("/source.wav"),
                    size_bytes: 20,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            Ok::<_, CatalogError>(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            })
        })
        .expect("asset registers");
    let revision = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(
                transaction,
                asset.id,
                AdjustmentGraph::new(
                    1_000,
                    0,
                    1_000,
                    0,
                    0,
                    AdjustmentEffects::new(FadeCurves::default(), 0, 0),
                )
                .expect("graph validates"),
                2,
            )
        })
        .expect("adjustment records");
    let copy = catalog
        .with_transaction(|transaction| {
            crate::create_rendered_spectral_working_copy(
                transaction,
                crate::CreateRenderedSpectralWorkingCopy {
                    asset_id: asset.id,
                    parent_adjustment_revision_id: revision.revision_id,
                    parent_render_content_hash: ContentHash::new([7; 32]),
                    created_at_millis: 3,
                },
            )
        })
        .expect("working copy records");
    let evidence = RecordRenderExport {
        asset_id: asset.id,
        adjustment_revision_id: revision.revision_id,
        output_path: PathBuf::from("/exports/repaired.wav"),
        format: RenderExportFormat::WavPcm24,
        sample_rate: 48_000,
        channel_count: 2,
        bit_depth: 24,
        frame_count: 48_000,
        content_hash: ContentHash::new([8; 32]),
        size_bytes: 288_044,
        integrated_lufs: -18.0,
        true_peak_dbtp: -1.0,
        created_at_millis: 4,
    };
    let record = catalog
        .with_transaction(|transaction| {
            record_rendered_spectral_working_copy_export(transaction, &evidence, copy.id)
        })
        .expect("working-copy export records");
    let snapshot = catalog
        .with_transaction(|transaction| {
            Ok::<_, CatalogError>(transaction.query_row(
                "SELECT working_copy_id, original_content_hash, parent_render_content_hash, \
                 working_render_content_hash, tile_manifest_json, tool_version \
                 FROM render_export_working_copy_provenance WHERE render_export_id = ?1",
                [record.id],
                |row| {
                    Ok::<_, rusqlite::Error>((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )?)
        })
        .expect("provenance snapshot reads");
    assert_eq!(snapshot.0, copy.id);
    assert_eq!(snapshot.1, ContentHash::new([4; 32]).to_string());
    assert_eq!(snapshot.2, ContentHash::new([7; 32]).to_string());
    assert_eq!(snapshot.3, ContentHash::new([7; 32]).to_string());
    assert_eq!(snapshot.4, r#"{"schema":1,"operations":[]}"#);
    assert_eq!(snapshot.5, "rendered-spectral-working-copy-v1");
    let _ = std::fs::remove_dir_all(root);
}
