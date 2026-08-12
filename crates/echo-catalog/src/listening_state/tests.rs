use std::path::Path;

use echo_domain::ContentHash;

use super::*;
use crate::{AssetAffinity, AssetRegistrationInput, RegisterAsset};

fn fixture_asset(transaction: &Transaction<'_>) -> Result<AssetId, CatalogError> {
    let registered = crate::register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([0x71; 32]),
            path: Path::new("/recordings/long-memory.wav"),
            size_bytes: 1,
            codec: Some("pcm"),
            duration_millis: Some(180_000),
            recorded_at_millis: None,
            imported_at_millis: 1,
        },
    )?;
    Ok(match registered {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    })
}

#[test]
fn listening_progress_is_bounded_and_preserves_affinity() {
    let root =
        std::env::temp_dir().join(format!("echo-listening-continuity-{}", std::process::id()));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            let asset_id = fixture_asset(transaction)?;
            assert_eq!(
                asset_listening_state(transaction, asset_id)?,
                AssetListeningState::default()
            );
            crate::set_asset_affinity(
                transaction,
                asset_id,
                AssetAffinity {
                    liked: true,
                    rating: 4,
                },
                2,
            )?;

            let short =
                record_asset_listening_progress(transaction, asset_id, 20_000, 0, 30_000, 10)?;
            assert_eq!(short.resume_position_millis, 0);
            let resumable =
                record_asset_listening_progress(transaction, asset_id, 72_000, 5_000, 155_000, 20)?;
            assert_eq!(resumable.resume_position_millis, 72_000);
            let completed = record_asset_listening_progress(
                transaction,
                asset_id,
                150_000,
                5_000,
                155_000,
                30,
            )?;
            assert_eq!(completed.resume_position_millis, 0);
            assert_eq!(completed.last_listened_at_millis, 30);

            assert_eq!(
                crate::asset_affinity(transaction, asset_id)?,
                AssetAffinity {
                    liked: true,
                    rating: 4
                }
            );
            crate::set_asset_affinity(
                transaction,
                asset_id,
                AssetAffinity {
                    liked: false,
                    rating: 5,
                },
                40,
            )?;
            assert_eq!(asset_listening_state(transaction, asset_id)?, completed);
            Ok(())
        })
        .expect("listening continuity round trips");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn invalid_listening_ranges_fail_closed() {
    let root = std::env::temp_dir().join(format!("echo-listening-invalid-{}", std::process::id()));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            let asset_id = fixture_asset(transaction)?;
            assert!(record_asset_listening_progress(transaction, asset_id, 0, 10, 10, 1).is_err());
            assert!(record_asset_listening_progress(transaction, asset_id, 9, 10, 20, 1).is_err());
            assert!(record_asset_listening_progress(transaction, asset_id, 21, 10, 20, 1).is_err());
            assert!(
                record_asset_listening_progress(transaction, asset_id, 15, 10, 20, -1).is_err()
            );
            Ok(())
        })
        .expect("invalid ranges are rejected");
    let _ = std::fs::remove_dir_all(root);
}
