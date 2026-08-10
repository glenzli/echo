use super::*;
use crate::open_catalog;

fn fixture(name: &str) -> (std::path::PathBuf, crate::Catalog) {
    let root = std::env::temp_dir().join(format!("echo-job-{}-{name}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
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

    let second = catalog
        .with_transaction(|transaction| claim_next_job(transaction, 102))
        .expect("claim");
    assert_eq!(second.as_ref().map(|job| job.id.as_str()), Some("job-1"));
    drop(second);
    let recovered = catalog
        .with_transaction(|transaction| recover_interrupted_jobs(transaction, 200))
        .expect("recover");
    assert_eq!(recovered, 1);
    let retried = catalog
        .with_transaction(|transaction| claim_next_job(transaction, 201))
        .expect("claim");
    assert_eq!(retried.as_ref().map(|job| job.id.as_str()), Some("job-1"));
    let attempts: i64 = catalog
        .with_transaction(|transaction| {
            transaction
                .query_row("SELECT attempts FROM jobs WHERE id = 'job-1'", [], |row| {
                    row.get(0)
                })
                .map_err(crate::CatalogError::from)
        })
        .expect("read attempts");
    assert_eq!(attempts, 2);
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
            )?;
            enqueue_job(
                transaction,
                "b",
                JobKind::Contextual,
                &serde_json::json!({ "asset_id": "x" }),
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
fn structural_jobs_are_claimed_before_progressive_analysis() {
    let (root, catalog) = fixture("priority");
    catalog
        .with_transaction(|transaction| {
            enqueue_job(
                transaction,
                "old-transcript",
                JobKind::Transcribe,
                &serde_json::json!({ "asset_id": "x" }),
                0,
            )?;
            enqueue_job(
                transaction,
                "events",
                JobKind::DetectAudioEvents,
                &serde_json::json!({ "asset_id": "x" }),
                0,
            )?;
            enqueue_job(
                transaction,
                "waveform",
                JobKind::AnalyzeWaveform,
                &serde_json::json!({ "asset_id": "x" }),
                10,
            )?;
            enqueue_job(
                transaction,
                "import",
                JobKind::ImportFile,
                &FileJobPayload {
                    path: "/tmp/voice.wav".into(),
                }
                .encode(),
                10,
            )?;
            enqueue_job(
                transaction,
                "scan",
                JobKind::ScanRoot,
                &ScanRootJobPayload {
                    root: "/tmp".into(),
                }
                .encode(),
                10,
            )
        })
        .expect("enqueue");

    let mut claimed = Vec::new();
    for now in 20..25 {
        claimed.push(
            catalog
                .with_transaction(|transaction| claim_next_job(transaction, now))
                .expect("claim")
                .expect("pending job")
                .id,
        );
    }
    assert_eq!(
        claimed,
        ["scan", "import", "waveform", "old-transcript", "events"]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn failed_job_projection_reads_every_selected_column() {
    let (root, catalog) = fixture("failed-projection");
    catalog
        .with_transaction(|transaction| {
            enqueue_job(
                transaction,
                "failed",
                JobKind::Transcribe,
                &serde_json::json!({ "asset_id": "asset" }),
                10,
            )?;
            let _ = claim_next_job(transaction, 20)?;
            update_job_progress(transaction, "failed", 42, 30)?;
            fail_job(transaction, "failed", "worker unavailable", 40)
        })
        .expect("job fails");
    let jobs = catalog
        .with_transaction(list_failed_jobs)
        .expect("failed jobs read");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].progress, 42);
    assert_eq!(jobs[0].attempts, 1);
    assert_eq!(jobs[0].created_at_millis, 10);
    assert_eq!(jobs[0].updated_at_millis, 40);
    assert_eq!(jobs[0].error.as_deref(), Some("worker unavailable"));
    let _ = std::fs::remove_dir_all(root);
}
