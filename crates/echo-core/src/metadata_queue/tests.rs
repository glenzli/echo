use std::path::Path;

use echo_catalog::{
    AssetRegistrationInput, JobStats, RegisterAsset, job_stats, open_catalog, register_asset,
};
use echo_domain::ContentHash;

use super::*;

#[test]
fn metadata_backfill_is_idempotent() {
    let root = std::env::temp_dir().join(format!("echo-metadata-queue-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([8; 32]),
                    path: Path::new("/voices/legacy.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            assert!(matches!(registered, RegisterAsset::Created(_)));
            Ok(())
        })
        .expect("fixture writes");

    assert_eq!(
        enqueue_missing_source_metadata(&catalog, 20).expect("backfill queues"),
        1
    );
    assert_eq!(
        enqueue_missing_source_metadata(&catalog, 30).expect("backfill repeats"),
        1
    );
    let JobStats { pending, .. } = catalog.with_transaction(job_stats).expect("stats read");
    assert_eq!(pending, 1);
    let _ = std::fs::remove_dir_all(root);
}
