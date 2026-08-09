use std::path::Path;

use echo_domain::ContentHash;

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn affinity_defaults_and_round_trips() {
    let root = std::env::temp_dir().join(format!("echo-affinity-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([7; 32]),
                    path: Path::new("/voices/memory.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            Ok(match asset {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            })
        })
        .expect("fixture writes");

    assert_eq!(
        catalog
            .with_transaction(|transaction| asset_affinity(transaction, asset_id))
            .expect("default reads"),
        AssetAffinity::default()
    );
    catalog
        .with_transaction(|transaction| {
            set_asset_affinity(
                transaction,
                asset_id,
                AssetAffinity {
                    liked: true,
                    rating: 4,
                },
                20,
            )
        })
        .expect("affinity writes");
    assert_eq!(
        catalog
            .with_transaction(|transaction| asset_affinity(transaction, asset_id))
            .expect("affinity reads"),
        AssetAffinity {
            liked: true,
            rating: 4
        }
    );
    let _ = std::fs::remove_dir_all(root);
}
