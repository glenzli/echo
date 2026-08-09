use std::path::Path;

use echo_domain::ContentHash;

use super::*;
use crate::{AssetRegistrationInput, open_catalog, register_asset};

#[test]
fn artifact_reference_round_trips_and_replaces() {
    let root = std::env::temp_dir().join(format!("echo-derived-artifact-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let registered = catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([1; 32]),
                    path: Path::new("/recording.wav"),
                    size_bytes: 10,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )
        })
        .expect("register");
    let asset_id = match registered {
        crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => asset.id,
    };
    let mut expected = DerivedArtifactRecord {
        asset_id,
        kind: DerivedArtifactKind::WaveformPyramid,
        schema_version: 1,
        content_hash: ContentHash::new([2; 32]),
        size_bytes: 20,
        created_at_millis: 2,
    };
    catalog
        .with_transaction(|transaction| upsert_derived_artifact(transaction, &expected))
        .expect("publish");
    let found = catalog
        .with_transaction(|transaction| {
            find_derived_artifact(
                transaction,
                asset_id,
                DerivedArtifactKind::WaveformPyramid,
                1,
            )
        })
        .expect("read");
    assert_eq!(found, Some(expected.clone()));

    expected.content_hash = ContentHash::new([3; 32]);
    expected.size_bytes = 30;
    catalog
        .with_transaction(|transaction| upsert_derived_artifact(transaction, &expected))
        .expect("replace");
    let found = catalog
        .with_transaction(|transaction| {
            find_derived_artifact(
                transaction,
                asset_id,
                DerivedArtifactKind::WaveformPyramid,
                1,
            )
        })
        .expect("read");
    assert_eq!(found, Some(expected));
    let _ = std::fs::remove_dir_all(root);
}
