use echo_catalog::{
    AppendAnalysisRecord, AssetRegistrationInput, JobKind, RegisterAsset, fail_job, open_catalog,
    record_analysis, register_asset,
};
use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;

#[test]
fn clap_index_revision_is_stable() {
    assert_eq!(super::INDEX_REVISION, 1);
}

#[test]
fn legacy_sdk_failure_is_recovered_once_without_losing_job_identity() {
    let (root, catalog, asset_id) = fixture("schema-recovery");
    enqueue_missing_documents(&catalog, 10).expect("queue");
    catalog
        .with_transaction(|transaction| {
            transaction.execute(
                "UPDATE jobs SET attempts = 1 WHERE id = ?1",
                [job_id(asset_id)],
            )?;
            fail_job(transaction, &job_id(asset_id), LEGACY_SCHEMA_ERROR, 20)
        })
        .expect("old SDK failure");

    enqueue_missing_documents(&catalog, 30).expect("recover");
    enqueue_missing_documents(&catalog, 40).expect("repeated startup");
    catalog
        .with_transaction(|transaction| {
            let job = job_by_id(transaction, &job_id(asset_id))?.expect("same job");
            assert_eq!(job.kind, JobKind::EmbedAudio);
            assert_eq!(job.state, JobState::Pending);
            assert_eq!(job.attempts, 1);
            assert_eq!(job.created_at_millis, 10);
            assert_eq!(job.updated_at_millis, 30);
            assert_eq!(job.payload["asset_id"], asset_id.to_string());
            assert_eq!(job.payload["clap_schema_retry"], true);
            assert!(job.error.is_none());
            assert_eq!(
                transaction
                    .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get::<_, i64>(0))?,
                1
            );
            fail_job(transaction, &job.id, LEGACY_SCHEMA_ERROR, 50)
        })
        .expect("inspect recovery and fail again");
    enqueue_missing_documents(&catalog, 60).expect("later scan");
    catalog
        .with_transaction(|transaction| {
            let job = job_by_id(transaction, &job_id(asset_id))?.expect("job");
            assert_eq!(job.state, JobState::Failed);
            assert_eq!(job.updated_at_millis, 50);
            Ok::<_, echo_catalog::CatalogError>(())
        })
        .expect("no retry loop");
    drop(catalog);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn schema_recovery_preserves_other_failures_cancellation_and_active_jobs() {
    for (label, state, error) in [
        (
            "other-failure",
            "failed",
            "InferenceTimeout: model timed out",
        ),
        ("cancelled", "cancelled", LEGACY_SCHEMA_ERROR),
        ("pending", "pending", LEGACY_SCHEMA_ERROR),
        ("running", "running", LEGACY_SCHEMA_ERROR),
        ("done", "done", LEGACY_SCHEMA_ERROR),
    ] {
        let (root, catalog, asset_id) = fixture(label);
        enqueue_missing_documents(&catalog, 10).expect("queue");
        let before = catalog
            .with_transaction(|transaction| {
                transaction.execute(
                    "UPDATE jobs SET state = ?2, error = ?3 WHERE id = ?1",
                    rusqlite::params![job_id(asset_id), state, error],
                )?;
                job_by_id(transaction, &job_id(asset_id))
            })
            .expect("fixture state");
        enqueue_missing_documents(&catalog, 30).expect("scan");
        let after = catalog
            .with_transaction(|transaction| job_by_id(transaction, &job_id(asset_id)))
            .expect("read state");
        assert_eq!(before, after, "{label} must remain unchanged");
        drop(catalog);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}

#[test]
fn schema_recovery_does_not_rebuild_an_existing_audio_vector() {
    let (root, catalog, asset_id) = fixture("indexed");
    enqueue_missing_documents(&catalog, 10).expect("queue");
    catalog
        .with_transaction(|transaction| {
            fail_job(transaction, &job_id(asset_id), LEGACY_SCHEMA_ERROR, 20)?;
            let source = list_audio_sources_needing_embedding(transaction)?.remove(0);
            let mut vector = vec![0.0; 512];
            vector[0] = 1.0;
            upsert_audio_semantic_segment(
                transaction,
                &source,
                "fixture-clap-space",
                &vector,
                &serde_json::json!({}),
                25,
            )
        })
        .expect("existing vector");
    assert_eq!(enqueue_missing_documents(&catalog, 30).expect("scan"), 0);
    catalog
        .with_transaction(|transaction| {
            let job = job_by_id(transaction, &job_id(asset_id))?.expect("job");
            assert_eq!(job.state, JobState::Failed);
            assert!(job.payload.get("clap_schema_retry").is_none());
            Ok::<_, echo_catalog::CatalogError>(())
        })
        .expect("no redundant inference");
    drop(catalog);
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn fixture(label: &str) -> (std::path::PathBuf, Catalog, AssetId) {
    let root =
        std::env::temp_dir().join(format!("echo-clap-recovery-{}-{label}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog");
    let asset_id = catalog
        .with_transaction(|transaction| {
            let asset = match register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([93; 32]),
                    path: &root.join("rain.wav"),
                    size_bytes: 1024,
                    codec: Some("pcm"),
                    duration_millis: Some(8000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            };
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({"text": ""}),
                        ModelIdentity::new("fixture-asr".into(), "1".into()),
                        None,
                        2,
                    ),
                },
            )?;
            Ok::<_, echo_catalog::CatalogError>(asset.id)
        })
        .expect("empty transcript source");
    (root, catalog, asset_id)
}
