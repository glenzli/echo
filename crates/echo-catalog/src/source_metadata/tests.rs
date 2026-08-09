use std::path::Path;

use echo_domain::ContentHash;

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn source_metadata_round_trips_through_the_sound_wall_projection() {
    let root = std::env::temp_dir().join(format!("echo-source-metadata-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([9; 32]),
                    path: Path::new("/voices/phone.m4a"),
                    size_bytes: 100,
                    codec: Some("aac"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: Some(20),
                    imported_at_millis: 30,
                },
            )?;
            let asset_id = match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            record_source_metadata(
                transaction,
                asset_id,
                &SourceMetadata {
                    container_format: "mov,mp4,m4a".into(),
                    sample_rate: 48_000,
                    channel_count: 1,
                    entries: vec![SourceMetadataEntry {
                        key: "location".into(),
                        value: "+39.9042+116.4074/".into(),
                    }],
                },
                Some(20),
            )?;
            Ok(asset_id)
        })
        .expect("fixture writes");

    let assets = catalog
        .with_transaction(crate::list_audio_space)
        .expect("projection reads");
    let projected = assets
        .into_iter()
        .find(|asset| asset.id == asset_id.to_string())
        .expect("asset projects");
    assert_eq!(
        projected.source_metadata,
        Some(SourceMetadata {
            container_format: "mov,mp4,m4a".into(),
            sample_rate: 48_000,
            channel_count: 1,
            entries: vec![SourceMetadataEntry {
                key: "location".into(),
                value: "+39.9042+116.4074/".into(),
            }],
        })
    );
    assert!(
        catalog
            .with_transaction(list_assets_missing_source_metadata)
            .expect("missing projection reads")
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(root);
}
