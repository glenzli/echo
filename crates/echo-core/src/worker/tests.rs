use std::path::Path;

use echo_catalog::{
    AssetLookup, AssetRegistrationInput, ClaimedJob, JobKind, JobState, enqueue_job,
    find_by_content_hash, inference_run, job_by_id, open_catalog, query_analysis, record_analysis,
    register_asset,
};
use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;

#[test]
fn worker_state_revision_advances_when_a_job_reaches_terminal_state() {
    let root =
        std::env::temp_dir().join(format!("echo-worker-state-revision-{}", std::process::id()));
    let catalog = Arc::new(open_catalog(&root.join("catalog.sqlite")).expect("catalog opens"));
    let pool = WorkerPool::start(
        &catalog,
        &WorkerConfig {
            cache_root: root.join("cache"),
            infer_runtime: crate::InferRuntimeConfig {
                base_url: "http://127.0.0.1:1".to_owned(),
                credential_path: root.join("missing-infer-runtime.token"),
            },
        },
        1,
    )
    .expect("workers start");
    let initial_revision = pool.state_revision();
    catalog
        .with_transaction(|transaction| {
            enqueue_job(
                transaction,
                "invalid-worker-fixture",
                JobKind::Transcribe,
                &serde_json::json!({ "asset_id": "not-an-asset-id" }),
                10,
            )
        })
        .expect("job enqueues");

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while pool.state_revision() < initial_revision + 2 && std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(pool.state_revision() >= initial_revision + 2);
    pool.stop();

    let job = catalog
        .with_transaction(|transaction| job_by_id(transaction, "invalid-worker-fixture"))
        .expect("job reads")
        .expect("job exists");
    assert_eq!(job.state, JobState::Failed);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn runtime_completion_links_the_local_job_to_sanitized_provenance() {
    let root = std::env::temp_dir().join(format!("echo-worker-runtime-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([19; 32]),
                    path: Path::new("/voices/runtime-worker.wav"),
                    size_bytes: 128,
                    codec: Some("pcm"),
                    duration_millis: Some(800),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let AssetLookup::Found(asset) =
                find_by_content_hash(transaction, ContentHash::new([19; 32]))?
            else {
                panic!("asset exists")
            };
            echo_catalog::enqueue_job(
                transaction,
                "transcribe-worker",
                JobKind::Transcribe,
                &serde_json::json!({ "asset_id": asset.id.to_string() }),
                2,
            )?;
            Ok(asset.id)
        })
        .expect("fixture writes");
    let local_job = ClaimedJob {
        id: "transcribe-worker".to_owned(),
        kind: JobKind::Transcribe,
        payload: serde_json::json!({ "asset_id": asset_id.to_string() }),
    };
    let provenance = crate::RuntimeProvenance {
        contract_version: crate::EXPECTED_CONTRACT_VERSION.to_owned(),
        job: crate::RuntimeJobSnapshot {
            id: "runtime-job-1".to_owned(),
            consumer_core_contract: crate::EXPECTED_CONTRACT_VERSION.to_owned(),
            capability_contract: Some("infer.audio.transcription@20260814.1".to_owned()),
            app_id: "echo".to_owned(),
            intent: crate::TRANSCRIPTION_INTENT.to_owned(),
            provider: "mlx-audio-local".to_owned(),
            deployment: "mlx_qwen3_asr_1_7b".to_owned(),
            model_profile: "qwen3_asr_1_7b".to_owned(),
            model_build: "qwen3_asr_1_7b_8bit".to_owned(),
            physical_model: "Qwen3-ASR-1.7B".to_owned(),
            placement: "local".to_owned(),
            capability_level: "foundational".to_owned(),
            evaluation_status: "provisional".to_owned(),
            resource_class: "standard".to_owned(),
            state: "succeeded".to_owned(),
            policy: "local-first".to_owned(),
            priority: "background".to_owned(),
            constraints: crate::RuntimeJobConstraints::default(),
            routing: crate::RuntimeRoutingDecision::default(),
            attempts: vec![],
        },
    };

    record_inference_submitting(&catalog, &local_job, asset_id, crate::TRANSCRIPTION_INTENT)
        .expect("submission records");
    record_inference_success(&catalog, &local_job, asset_id, &provenance)
        .expect("completion records");

    let run = catalog
        .with_transaction(|transaction| inference_run(transaction, "transcribe-worker"))
        .expect("run reads")
        .expect("run exists");
    assert_eq!(run.runtime_job_id.as_deref(), Some("runtime-job-1"));
    assert_eq!(run.state, InferenceRunState::Succeeded);
    assert_eq!(run.snapshot.unwrap()["physical_model"], "Qwen3-ASR-1.7B");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn audio_event_job_without_an_available_runtime_fails_without_publishing_evidence() {
    let root =
        std::env::temp_dir().join(format!("echo-worker-audio-events-{}", std::process::id()));
    let source = crate::infer_runtime::tests::audio_fixture();
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([23; 32]),
                    path: &source,
                    size_bytes: std::fs::metadata(&source).expect("source stats").len(),
                    codec: Some("pcm"),
                    duration_millis: Some(2_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let asset_id = match registered {
                echo_catalog::RegisterAsset::Created(asset)
                | echo_catalog::RegisterAsset::Existed(asset) => asset.id,
            };
            record_analysis(
                transaction,
                &echo_catalog::AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "" }),
                        ModelIdentity::new("fixture-asr".into(), "1".into()),
                        None,
                        2,
                    ),
                },
            )?;
            echo_catalog::enqueue_job(
                transaction,
                "detect-audio-events-worker",
                JobKind::DetectAudioEvents,
                &serde_json::json!({ "asset_id": asset_id.to_string() }),
                3,
            )?;
            Ok(asset_id)
        })
        .expect("fixture writes");
    let job = ClaimedJob {
        id: "detect-audio-events-worker".to_owned(),
        kind: JobKind::DetectAudioEvents,
        payload: serde_json::json!({ "asset_id": asset_id.to_string() }),
    };
    let config = WorkerConfig {
        cache_root: root.join("cache"),
        infer_runtime: crate::InferRuntimeConfig {
            base_url: String::new(),
            credential_path: root.join("missing-infer-runtime.token"),
        },
    };

    let error = dispatch_analysis(&catalog, &config, &job).expect_err("missing credential fails");
    assert_eq!(error.kind, crate::CoreErrorKind::InferenceUnavailable);

    let records = catalog
        .with_transaction(|transaction| {
            query_analysis(transaction, asset_id).map_err(|error| {
                echo_catalog::CatalogError::new(
                    echo_catalog::CatalogErrorKind::Other,
                    error.to_string(),
                )
            })
        })
        .expect("analysis reads");
    assert!(!records.iter().any(|record| matches!(
        record.kind,
        AnalysisKind::AudioEvents | AnalysisKind::Contextual
    )));
    let run = catalog
        .with_transaction(|transaction| inference_run(transaction, &job.id))
        .expect("inference run reads")
        .expect("inference run exists");
    assert_eq!(run.state, InferenceRunState::Failed);
    assert_eq!(run.intent, crate::AUDIO_EVENT_DETECTION_INTENT);
    assert_eq!(run.error_code.as_deref(), Some("runtime_unavailable"));
    assert!(run.snapshot.is_none());

    std::fs::remove_file(source).expect("source removes");
    let _ = std::fs::remove_dir_all(root);
}
