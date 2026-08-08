//! Job queue facade contracts: claim/recovery semantics and scan-root
//! idempotency.

use crate::{
    AssetLookup, AssetRegistrationInput, Catalog, CatalogError, CatalogErrorKind, JobKind,
    ScanRootJobPayload, add_scan_root, claim_next_job, complete_job, enqueue_job, fail_job,
    find_by_content_hash, job_stats, list_scan_roots, mark_asset_missing, open_catalog,
    recover_interrupted_jobs, register_asset, relink_asset_by_hash, remove_scan_root,
};
use echo_domain::ContentHash;
use std::path::Path;
use std::path::PathBuf;

fn fixture(name: &str) -> (PathBuf, Catalog) {
    let root =
        std::env::temp_dir().join(format!("echo-catalog-jobs-{}-{name}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    (root, catalog)
}

#[test]
fn jobs_claim_in_order_and_recover_after_crash() {
    let (root, catalog) = fixture("recover");
    for index in 0..3 {
        catalog
            .with_transaction(|transaction| {
                enqueue_job(
                    transaction,
                    &format!("job-{index}"),
                    JobKind::ImportFile,
                    &serde_json::json!({ "path": format!("/tmp/f{index}.wav") }),
                    index,
                )
            })
            .expect("enqueue");
    }
    let first = catalog
        .with_transaction(|transaction| claim_next_job(transaction, 100))
        .expect("claim");
    assert_eq!(first.as_ref().map(|job| job.id.as_str()), Some("job-0"));
    catalog
        .with_transaction(|transaction| complete_job(transaction, "job-0", 101))
        .expect("complete");

    // Simulated crash: job-1 was claimed but never finished.
    let second = catalog
        .with_transaction(|transaction| claim_next_job(transaction, 102))
        .expect("claim");
    assert_eq!(second.as_ref().map(|job| job.id.as_str()), Some("job-1"));
    drop(second);

    let recovered = catalog
        .with_transaction(|transaction| recover_interrupted_jobs(transaction, 200))
        .expect("recover");
    assert_eq!(recovered, 1, "job-1 resets to pending");

    let retried = catalog
        .with_transaction(|transaction| claim_next_job(transaction, 201))
        .expect("claim");
    assert_eq!(retried.as_ref().map(|job| job.id.as_str()), Some("job-1"));
    let _ = retried;
    let attempts: i64 = catalog
        .with_transaction(|transaction| {
            transaction
                .query_row("SELECT attempts FROM jobs WHERE id = 'job-1'", [], |row| {
                    row.get(0)
                })
                .map_err(|error| CatalogError::new(CatalogErrorKind::Other, error.to_string()))
        })
        .expect("read attempts");
    assert_eq!(attempts, 2, "attempt counter survives recovery");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn failed_jobs_record_errors_and_stats_aggregate() {
    let (root, catalog) = fixture("stats");
    catalog
        .with_transaction(|transaction| {
            enqueue_job(
                transaction,
                "a",
                JobKind::Transcribe,
                &serde_json::json!({ "asset_id": "x" }),
                0,
            )
        })
        .expect("enqueue");
    catalog
        .with_transaction(|transaction| {
            enqueue_job(
                transaction,
                "b",
                JobKind::ScanRoot,
                &ScanRootJobPayload {
                    root: "/tmp".into(),
                }
                .encode(),
                0,
            )
        })
        .expect("enqueue");
    catalog
        .with_transaction(|transaction| claim_next_job(transaction, 1).map(|_| ()))
        .expect("claim");
    catalog
        .with_transaction(|transaction| fail_job(transaction, "a", "model missing", 2))
        .expect("fail");
    let stats = catalog.with_transaction(job_stats).expect("stats");
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.failed, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn scan_roots_round_trip_and_remove() {
    let (root, catalog) = fixture("roots");
    catalog
        .with_transaction(|transaction| add_scan_root(transaction, Path::new("/voices"), 1))
        .expect("add");
    catalog
        .with_transaction(|transaction| add_scan_root(transaction, Path::new("/voices"), 2))
        .expect("duplicate add is idempotent");
    let roots = catalog.with_transaction(list_scan_roots).expect("list");
    assert_eq!(roots.len(), 1);
    catalog
        .with_transaction(|transaction| remove_scan_root(transaction, roots[0].id))
        .expect("remove");
    let roots = catalog.with_transaction(list_scan_roots).expect("list");
    assert!(roots.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn relink_restores_missing_assets_by_hash() {
    let (root, catalog) = fixture("relink");
    let hash = ContentHash::new([5; 32]);
    catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: hash,
                    path: Path::new("/old/lost.wav"),
                    size_bytes: 100,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("register");

    // Find the real id through the hash lookup.
    let id = catalog
        .with_transaction(|transaction| -> Result<String, CatalogError> {
            match find_by_content_hash(transaction, hash)? {
                AssetLookup::Found(asset) => Ok(asset.id.to_string()),
                AssetLookup::NotFound => panic!("must exist"),
            }
        })
        .expect("find id");
    catalog
        .with_transaction(|transaction| mark_asset_missing(transaction, &id))
        .expect("mark missing");

    let relinked = catalog
        .with_transaction(|transaction| {
            relink_asset_by_hash(transaction, &hash.to_string(), Path::new("/new/found.wav"))
        })
        .expect("relink");
    assert!(relinked, "missing asset relinks by hash");
    let asset = catalog
        .with_transaction(
            |transaction| -> Result<echo_domain::AudioAsset, CatalogError> {
                match find_by_content_hash(transaction, hash)? {
                    AssetLookup::Found(asset) => Ok(asset),
                    AssetLookup::NotFound => panic!("must exist"),
                }
            },
        )
        .expect("find");
    assert_eq!(
        asset.original.path,
        std::path::PathBuf::from("/new/found.wav")
    );
    let _ = std::fs::remove_dir_all(root);
}
