use std::path::Path;

use echo_domain::ContentHash;

use super::*;
use crate::{
    AssetRegistrationInput, InferenceRunState, JobKind, LongAudioSegmentPlan, UpsertInferenceRun,
    enqueue_job, ensure_long_audio_plan, fail_job, open_catalog, record_analysis, register_asset,
    upsert_inference_run,
};

const CONTEXTUAL_SCHEMA: u32 = 3;
const CONTEXTUAL_REVISION: u32 = 1;
const LONG_AUDIO_PLAN: u32 = 1;

#[test]
fn projects_empty_speech_through_sound_events_before_completion() {
    let root = fixture_root("echo-analysis-status-events");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = register_fixture_asset(transaction, 91, 1)?;
            record_analysis(
                transaction,
                &crate::AppendAnalysisRecord {
                    asset_id: asset,
                    record: echo_domain::AnalysisRecord {
                        kind: echo_domain::AnalysisKind::Transcript,
                        value: serde_json::json!({"text": "", "segments": []}),
                        model: echo_domain::ModelIdentity::new("asr".to_owned(), "1".to_owned()),
                        confidence: None,
                        recorded_at_millis: 2,
                    },
                },
            )?;
            enqueue_job(
                transaction,
                &format!("detect-audio-events-v1-{asset}"),
                JobKind::DetectAudioEvents,
                &serde_json::json!({"asset_id": asset.to_string()}),
                3,
            )?;
            ensure_long_audio_plan(
                transaction,
                asset,
                LONG_AUDIO_PLAN,
                &[LongAudioSegmentPlan {
                    index: 0,
                    start_millis: 0,
                    end_millis: 500,
                }],
                3,
            )?;
            Ok(asset)
        })
        .expect("fixture writes");

    let status = statuses(&catalog)
        .into_iter()
        .find(|status| status.asset_id == asset)
        .expect("asset status exists");
    assert_eq!(status.stage, AnalysisStage::SoundEvents);
    assert_eq!(status.state_str(), "pending");
    assert_eq!(status.recovery, AnalysisRecoveryMode::None);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn transient_failure_is_automatic_but_contract_failure_is_manual() {
    let root = fixture_root("echo-analysis-recovery-policy");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (automatic, manual) = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let automatic = register_fixture_asset(transaction, 92, 1)?;
            let manual = register_fixture_asset(transaction, 93, 2)?;
            install_failed_transcription(transaction, automatic, "runtime_unavailable", 3)?;
            install_failed_transcription(transaction, manual, "upstream_protocol", 4)?;
            Ok((automatic, manual))
        })
        .expect("fixtures write");

    let projected = statuses(&catalog);
    assert_eq!(
        status_for(&projected, automatic).recovery,
        AnalysisRecoveryMode::Automatic
    );
    assert_eq!(
        status_for(&projected, manual).recovery,
        AnalysisRecoveryMode::Manual
    );
    let automatic_count = catalog
        .with_transaction(|transaction| {
            automatic_analysis_recovery_count(
                transaction,
                CONTEXTUAL_SCHEMA,
                CONTEXTUAL_REVISION,
                LONG_AUDIO_PLAN,
            )
        })
        .expect("automatic count reads");
    assert_eq!(automatic_count, 1);

    let resumed = catalog
        .with_transaction(|transaction| {
            requeue_automatic_analysis(
                transaction,
                CONTEXTUAL_SCHEMA,
                CONTEXTUAL_REVISION,
                LONG_AUDIO_PLAN,
                10,
            )
        })
        .expect("automatic stage resumes");
    assert_eq!(resumed, 1);
    let projected_after_auto = statuses(&catalog);
    assert_eq!(
        status_for(&projected_after_auto, automatic).state_str(),
        "pending"
    );
    assert_eq!(
        status_for(&projected_after_auto, manual).state_str(),
        "failed"
    );
    let manually_resumed = catalog
        .with_transaction(|transaction| {
            requeue_manual_analysis(
                transaction,
                CONTEXTUAL_SCHEMA,
                CONTEXTUAL_REVISION,
                LONG_AUDIO_PLAN,
                11,
            )
        })
        .expect("manual stage resumes");
    assert_eq!(manually_resumed, 1);
    assert_eq!(
        status_for(&statuses(&catalog), manual).state_str(),
        "pending"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn missing_original_blocks_retry_without_erasing_failed_state() {
    let root = fixture_root("echo-analysis-recovery-source");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = register_fixture_asset(transaction, 94, 1)?;
            install_failed_transcription(transaction, asset, "runtime_unavailable", 3)?;
            transaction.execute(
                "UPDATE assets SET path_status = 'missing' WHERE id = ?1",
                [asset.to_string()],
            )?;
            Ok(asset)
        })
        .expect("fixture writes");
    let status = statuses(&catalog)
        .into_iter()
        .find(|status| status.asset_id == asset)
        .expect("asset status exists");
    assert_eq!(status.recovery, AnalysisRecoveryMode::Source);
    let resumed = catalog
        .with_transaction(|transaction| {
            requeue_automatic_analysis(
                transaction,
                CONTEXTUAL_SCHEMA,
                CONTEXTUAL_REVISION,
                LONG_AUDIO_PLAN,
                10,
            )
        })
        .expect("recovery pass succeeds");
    assert_eq!(resumed, 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn stale_transient_failure_does_not_requeue_after_evidence_advances() {
    let root = fixture_root("echo-analysis-recovery-current-stage");
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = register_fixture_asset(transaction, 95, 1)?;
            install_failed_transcription(transaction, asset, "runtime_unavailable", 3)?;
            record_analysis(
                transaction,
                &crate::AppendAnalysisRecord {
                    asset_id: asset,
                    record: echo_domain::AnalysisRecord {
                        kind: echo_domain::AnalysisKind::Transcript,
                        value: serde_json::json!({"text": "speech", "segments": []}),
                        model: echo_domain::ModelIdentity::new("asr".to_owned(), "1".to_owned()),
                        confidence: None,
                        recorded_at_millis: 4,
                    },
                },
            )?;
            Ok(())
        })
        .expect("fixture writes");

    let automatic_count = catalog
        .with_transaction(|transaction| {
            automatic_analysis_recovery_count(
                transaction,
                CONTEXTUAL_SCHEMA,
                CONTEXTUAL_REVISION,
                LONG_AUDIO_PLAN,
            )
        })
        .expect("automatic count reads");
    assert_eq!(automatic_count, 0);
    let _ = std::fs::remove_dir_all(root);
}

fn statuses(catalog: &crate::Catalog) -> Vec<AssetAnalysisStatus> {
    catalog
        .with_transaction(|transaction| {
            list_asset_analysis_statuses(
                transaction,
                CONTEXTUAL_SCHEMA,
                CONTEXTUAL_REVISION,
                LONG_AUDIO_PLAN,
            )
        })
        .expect("statuses read")
}

fn status_for(
    statuses: &[AssetAnalysisStatus],
    asset_id: echo_domain::AssetId,
) -> &AssetAnalysisStatus {
    statuses
        .iter()
        .find(|status| status.asset_id == asset_id)
        .expect("status exists")
}

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{name}-{}", std::process::id()))
}

fn register_fixture_asset(
    transaction: &rusqlite::Transaction<'_>,
    hash_byte: u8,
    imported_at_millis: i64,
) -> Result<echo_domain::AssetId, crate::CatalogError> {
    let registered = register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([hash_byte; 32]),
            path: Path::new("/voices/recovery.wav"),
            size_bytes: 64,
            codec: Some("pcm"),
            duration_millis: Some(500),
            recorded_at_millis: None,
            imported_at_millis,
        },
    )?;
    Ok(match registered {
        crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => asset.id,
    })
}

fn install_failed_transcription(
    transaction: &rusqlite::Transaction<'_>,
    asset_id: echo_domain::AssetId,
    error_code: &str,
    now_millis: i64,
) -> Result<(), crate::CatalogError> {
    let job_id = format!("transcribe-{asset_id}");
    enqueue_job(
        transaction,
        &job_id,
        JobKind::Transcribe,
        &serde_json::json!({"asset_id": asset_id.to_string()}),
        now_millis,
    )?;
    fail_job(transaction, &job_id, "sanitized failure", now_millis)?;
    upsert_inference_run(
        transaction,
        &UpsertInferenceRun {
            local_job_id: &job_id,
            asset_id,
            intent: "audio.transcribe",
            runtime_job_id: None,
            contract_version: "0.1.0-candidate.4",
            state: InferenceRunState::Failed,
            http_status: None,
            error_code: Some(error_code),
            snapshot: None,
            updated_at_millis: now_millis,
        },
    )
}
