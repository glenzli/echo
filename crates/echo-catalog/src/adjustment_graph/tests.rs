use std::path::Path;

use echo_domain::{AdjustmentGraph, ContentHash};

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn revisions_are_append_only_and_identical_saves_are_idempotent() {
    let root = std::env::temp_dir().join(format!("echo-adjustments-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([31; 32]),
                    path: Path::new("/voices/edit.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            Ok(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            })
        })
        .expect("fixture writes");
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, asset_id))
            .expect("untouched graph reads"),
        None
    );

    let graph =
        AdjustmentGraph::new(10_000, 1_000, 9_000, 250, 500, -300).expect("graph validates");
    let first = catalog
        .with_transaction(|transaction| record_adjustment_graph(transaction, asset_id, graph, 20))
        .expect("first revision writes");
    let duplicate = catalog
        .with_transaction(|transaction| record_adjustment_graph(transaction, asset_id, graph, 30))
        .expect("duplicate save reads current");
    assert_eq!(duplicate, first);

    let second_graph =
        AdjustmentGraph::new(10_000, 2_000, 8_000, 100, 100, 0).expect("second graph validates");
    let second = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, asset_id, second_graph, 40)
        })
        .expect("second revision writes");
    assert!(second.revision_id > first.revision_id);
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, asset_id))
            .expect("latest reads")
            .expect("revision exists")
            .graph,
        second_graph
    );
    let _ = std::fs::remove_dir_all(root);
}
