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
    assert_eq!(records, vec![record]);

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
