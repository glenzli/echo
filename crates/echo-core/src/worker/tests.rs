use std::path::Path;

use echo_catalog::{
    AssetLookup, AssetRegistrationInput, ClaimedJob, JobKind, find_by_content_hash, inference_run,
    open_catalog, register_asset,
};
use echo_domain::ContentHash;

use super::*;

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
            app_id: "echo".to_owned(),
            intent: crate::TRANSCRIPTION_INTENT.to_owned(),
            provider: "mlx-audio-local".to_owned(),
            deployment: "mlx_qwen3_asr_1_7b".to_owned(),
            model_profile: "qwen3_asr_1_7b".to_owned(),
            model_build: "qwen3_asr_1_7b_8bit".to_owned(),
            physical_model: "Qwen3-ASR-1.7B".to_owned(),
            placement: "local".to_owned(),
            quality_grade: "basic".to_owned(),
            rating_status: "provisional".to_owned(),
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
