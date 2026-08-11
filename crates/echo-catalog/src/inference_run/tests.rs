use std::path::Path;

use echo_domain::{AssetId, ContentHash};

use super::*;
use crate::{
    AssetLookup, AssetRegistrationInput, Catalog, find_by_content_hash, open_catalog,
    register_asset,
};

fn fixture(name: &str, hash_byte: u8, job_id: &str) -> (std::path::PathBuf, Catalog, AssetId) {
    let root = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([hash_byte; 32]),
                    path: Path::new("/voices/runtime.wav"),
                    size_bytes: 64,
                    codec: Some("pcm"),
                    duration_millis: Some(500),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let AssetLookup::Found(asset) =
                find_by_content_hash(transaction, ContentHash::new([hash_byte; 32]))?
            else {
                panic!("asset exists")
            };
            crate::enqueue_job(
                transaction,
                job_id,
                crate::JobKind::Transcribe,
                &serde_json::json!({ "asset_id": asset.id.to_string() }),
                2,
            )?;
            Ok(asset.id)
        })
        .expect("fixture writes");
    (root, catalog, asset_id)
}

#[test]
fn runtime_projection_replaces_submission_with_auditable_completion() {
    let (root, catalog, asset_id) = fixture("echo-inference-run", 8, "transcribe-runtime");

    catalog
        .with_transaction(|transaction| {
            upsert_inference_run(
                transaction,
                &UpsertInferenceRun {
                    local_job_id: "transcribe-runtime",
                    asset_id,
                    intent: "audio.transcribe",
                    runtime_job_id: None,
                    contract_version: "0.1.0-candidate.1",
                    state: InferenceRunState::Submitting,
                    http_status: None,
                    error_code: None,
                    snapshot: None,
                    updated_at_millis: 3,
                },
            )
        })
        .expect("submission records");

    let snapshot = serde_json::json!({
        "id": "audio_example",
        "state": "succeeded",
        "attempts": [{ "number": 0, "outcome": "succeeded" }]
    });
    catalog
        .with_transaction(|transaction| {
            upsert_inference_run(
                transaction,
                &UpsertInferenceRun {
                    local_job_id: "transcribe-runtime",
                    asset_id,
                    intent: "audio.transcribe",
                    runtime_job_id: Some("audio_example"),
                    contract_version: "0.1.0-candidate.1",
                    state: InferenceRunState::Succeeded,
                    http_status: Some(200),
                    error_code: None,
                    snapshot: Some(&snapshot),
                    updated_at_millis: 4,
                },
            )
        })
        .expect("completion replaces submission");

    let run = catalog
        .with_transaction(|transaction| inference_run(transaction, "transcribe-runtime"))
        .expect("run reads")
        .expect("run exists");
    assert_eq!(run.asset_id, asset_id);
    assert_eq!(run.runtime_job_id.as_deref(), Some("audio_example"));
    assert_eq!(run.state, InferenceRunState::Succeeded);
    assert_eq!(run.snapshot, Some(snapshot));

    let _ = std::fs::remove_dir_all(root);
}
