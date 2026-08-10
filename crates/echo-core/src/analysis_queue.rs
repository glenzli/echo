//! Default background-analysis admission.
//!
//! Import and startup call this owner to persist product-level inference
//! intent. Model execution, routing, retry, and resource lifecycle remain with
//! Infer Runtime.

use echo_catalog::{
    Catalog, JobKind, complete_job, enqueue_job, job_by_id, list_assets_missing_analysis,
    list_assets_with_alignment_missing_current_contextual,
    list_assets_with_empty_latest_transcript,
    list_assets_with_empty_transcript_missing_audio_events,
    list_assets_with_nonempty_transcript_missing_alignment,
};
use echo_domain::{AnalysisKind, AssetId};
use rusqlite::Transaction;

use crate::error::CoreError;

/// Persists the default transcription intent for one imported asset.
pub(crate) fn enqueue_transcription(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    enqueue_job(
        transaction,
        &transcription_job_id(asset_id),
        JobKind::Transcribe,
        &serde_json::json!({ "asset_id": asset_id.to_string() }),
        now_millis,
    )
}

/// Ensures every present asset without transcript evidence has one durable
/// transcription intent. Existing pending, running, completed, or failed jobs
/// keep their terminal policy because ordinary enqueue is idempotent.
pub(crate) fn enqueue_missing_transcriptions(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_missing_analysis(transaction, AnalysisKind::Transcript)?;
            for asset_id in &assets {
                enqueue_transcription(transaction, *asset_id, now_millis)?;
            }
            Ok(u64::try_from(assets.len()).expect("asset count fits u64"))
        })
        .map_err(CoreError::from)
}

/// Persists forced alignment after transcript evidence exists.
pub(crate) fn enqueue_alignment(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    enqueue_job(
        transaction,
        &alignment_job_id(asset_id),
        JobKind::Align,
        &serde_json::json!({ "asset_id": asset_id.to_string() }),
        now_millis,
    )
}

/// Backfills durable alignment intents only where transcript evidence is
/// already present.
pub(crate) fn enqueue_missing_alignments(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_with_nonempty_transcript_missing_alignment(transaction)?;
            for asset_id in &assets {
                enqueue_alignment(transaction, *asset_id, now_millis)?;
            }
            Ok(u64::try_from(assets.len()).expect("asset count fits u64"))
        })
        .map_err(CoreError::from)
}

/// Persists sound-event detection after ASR produced valid empty text.
pub(crate) fn enqueue_audio_events(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    enqueue_job(
        transaction,
        &audio_event_job_id(asset_id),
        JobKind::DetectAudioEvents,
        &serde_json::json!({ "asset_id": asset_id.to_string() }),
        now_millis,
    )
}

/// Backfills sound-event intent only for present assets whose latest ASR
/// observation is valid and empty. Unknown or failed ASR never reaches this
/// admission path.
pub(crate) fn enqueue_missing_audio_events(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_with_empty_transcript_missing_audio_events(transaction)?;
            for asset_id in &assets {
                enqueue_audio_events(transaction, *asset_id, now_millis)?;
            }
            Ok(u64::try_from(assets.len()).expect("asset count fits u64"))
        })
        .map_err(CoreError::from)
}

/// Persists contextual understanding after alignment evidence exists.
pub(crate) fn enqueue_contextual(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    enqueue_job(
        transaction,
        &contextual_job_id(asset_id),
        JobKind::Contextual,
        &serde_json::json!({ "asset_id": asset_id.to_string() }),
        now_millis,
    )
}

/// Backfills durable contextual intents only for aligned, non-empty text.
pub(crate) fn enqueue_missing_contextual(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_with_alignment_missing_current_contextual(
                transaction,
                crate::CONTEXTUAL_SCHEMA_VERSION,
            )?;
            for asset_id in &assets {
                enqueue_contextual(transaction, *asset_id, now_millis)?;
            }
            Ok(u64::try_from(assets.len()).expect("asset count fits u64"))
        })
        .map_err(CoreError::from)
}

/// Treats forced alignment as not applicable when ASR produced a valid empty
/// transcript. This also cleans up failed compatibility-era alignment jobs.
pub(crate) fn settle_empty_transcript_alignments(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_with_empty_latest_transcript(transaction)?;
            let mut settled = 0_u64;
            for asset_id in assets {
                let job_id = alignment_job_id(asset_id);
                if job_by_id(transaction, &job_id)?.is_some() {
                    complete_job(transaction, &job_id, now_millis)?;
                    settled += 1;
                }
            }
            Ok(settled)
        })
        .map_err(CoreError::from)
}

fn transcription_job_id(asset_id: AssetId) -> String {
    format!("transcribe-{asset_id}")
}

pub(crate) fn alignment_job_id(asset_id: AssetId) -> String {
    format!("align-{asset_id}")
}

pub(crate) fn audio_event_job_id(asset_id: AssetId) -> String {
    format!("detect-audio-events-v1-{asset_id}")
}

#[must_use]
pub fn contextual_job_id(asset_id: AssetId) -> String {
    format!(
        "contextual-v{}-r{}-{asset_id}",
        crate::CONTEXTUAL_SCHEMA_VERSION,
        crate::CONTEXTUAL_JOB_REVISION,
    )
}

#[cfg(test)]
mod tests;
