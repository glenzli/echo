use std::path::Path;

use echo_domain::{AssetId, ContentHash};

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset};

const NOW_MILLIS: i64 = 1_786_622_400_000; // 2026-08-13 12:00:00 UTC
const PRIOR_SAME_DAY_MILLIS: i64 = 1_755_075_600_000; // 2025-08-13 09:00:00 UTC
const PRIOR_OTHER_DAY_MILLIS: i64 = 1_754_989_200_000; // 2025-08-12 09:00:00 UTC

fn register_fixture(
    transaction: &Transaction<'_>,
    seed: u8,
    recorded_at_millis: i64,
    imported_at_millis: i64,
) -> Result<AssetId, CatalogError> {
    let path = format!("/recordings/revisit-{seed}.wav");
    let registered = crate::register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([seed; 32]),
            path: Path::new(&path),
            size_bytes: 1_024,
            codec: Some("pcm"),
            duration_millis: Some(120_000),
            recorded_at_millis: Some(recorded_at_millis),
            imported_at_millis,
        },
    )?;
    Ok(match registered {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    })
}

#[test]
fn revisit_sections_are_bounded_disjoint_and_source_anchored() {
    let root = std::env::temp_dir().join(format!("echo-revisit-{}", std::process::id()));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            let resumable = register_fixture(transaction, 1, PRIOR_OTHER_DAY_MILLIS, 10)?;
            let completed = register_fixture(transaction, 2, PRIOR_OTHER_DAY_MILLIS, 20)?;
            let anniversary = register_fixture(transaction, 3, PRIOR_SAME_DAY_MILLIS, 30)?;
            let missing_anniversary = register_fixture(transaction, 4, PRIOR_SAME_DAY_MILLIS, 40)?;
            let newest = register_fixture(transaction, 5, PRIOR_OTHER_DAY_MILLIS, 50)?;

            crate::record_asset_listening_progress(
                transaction,
                resumable,
                72_000,
                0,
                120_000,
                NOW_MILLIS - 2,
            )?;
            crate::record_asset_listening_progress(
                transaction,
                completed,
                115_000,
                0,
                120_000,
                NOW_MILLIS - 1,
            )?;
            crate::mark_asset_missing(transaction, &missing_anniversary.to_string())?;

            let snapshot = revisit_snapshot(transaction, NOW_MILLIS)?;
            assert_eq!(snapshot.continue_listening_asset_ids, vec![resumable]);
            assert_eq!(snapshot.recently_listened_asset_ids, vec![completed]);
            assert_eq!(snapshot.on_this_day_asset_ids, vec![anniversary]);
            assert_eq!(snapshot.recently_added_asset_ids[0], newest);
            assert!(!snapshot.recently_added_asset_ids.contains(&resumable));
            assert!(!snapshot.recently_added_asset_ids.contains(&completed));
            assert!(
                !snapshot
                    .recently_added_asset_ids
                    .contains(&missing_anniversary)
            );
            Ok(())
        })
        .expect("Revisit projection succeeds");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn continue_listening_is_capped_before_projection() {
    let root = std::env::temp_dir().join(format!("echo-revisit-cap-{}", std::process::id()));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            for seed in 1_u8..=10 {
                let asset =
                    register_fixture(transaction, seed, PRIOR_OTHER_DAY_MILLIS, i64::from(seed))?;
                crate::record_asset_listening_progress(
                    transaction,
                    asset,
                    60_000,
                    0,
                    120_000,
                    NOW_MILLIS + i64::from(seed),
                )?;
            }
            let snapshot = revisit_snapshot(transaction, NOW_MILLIS + 20)?;
            assert_eq!(snapshot.continue_listening_asset_ids.len(), 8);
            Ok(())
        })
        .expect("bounded Revisit projection succeeds");
    let _ = std::fs::remove_dir_all(root);
}
