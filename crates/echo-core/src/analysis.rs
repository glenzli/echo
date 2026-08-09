//! Transcript and alignment evidence owned by Echo. Runtime transport and
//! scheduling remain in the dedicated Infer Runtime consumer.

use std::time::{SystemTime, UNIX_EPOCH};

use echo_catalog::{AppendAnalysisRecord, record_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::{
    error::{CoreError, CoreErrorKind},
    infer_runtime::{AlignmentPayload, RuntimeProvenance},
};

/// One transcribed segment with timestamps in seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    #[serde(alias = "start_time")]
    pub start: f64,
    #[serde(alias = "end_time")]
    pub end: f64,
    /// Word-level timestamps when the model provides them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<Vec<TranscriptWord>>,
}

/// Word-level timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptWord {
    pub text: String,
    #[serde(alias = "start_time")]
    pub start: f64,
    #[serde(alias = "end_time")]
    pub end: f64,
}

/// Canonical transcript payload produced by the worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptPayload {
    pub model: String,
    pub language: Option<String>,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeProvenance>,
}

/// Records a transcript as evidence for `asset_id`.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn record_transcript(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    payload: &TranscriptPayload,
    model_version: &str,
) -> Result<(), CoreError> {
    record_transcript_with_model(
        catalog,
        asset_id,
        payload,
        ModelIdentity::new("mlx/qwen3-asr".to_owned(), model_version.to_owned()),
    )
}

/// Records Runtime-produced transcript evidence using the routed physical
/// model and immutable model build from the accepted Job snapshot.
///
/// # Errors
///
/// Returns a protocol failure when provenance is absent and a catalog failure
/// when the evidence cannot be committed.
pub fn record_runtime_transcript(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    payload: &TranscriptPayload,
) -> Result<(), CoreError> {
    let provenance = payload.runtime.as_ref().ok_or_else(|| {
        CoreError::new(
            CoreErrorKind::InferenceRejected,
            "Runtime transcript lacks accepted Job provenance",
        )
    })?;
    record_transcript_with_model(
        catalog,
        asset_id,
        payload,
        ModelIdentity::new(
            provenance.job.physical_model.clone(),
            provenance.job.model_build.clone(),
        ),
    )
}

fn record_transcript_with_model(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    payload: &TranscriptPayload,
    model: ModelIdentity,
) -> Result<(), CoreError> {
    let value = serde_json::to_value(payload).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode transcript: {error}"),
        )
    })?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        });
    catalog.with_transaction(|transaction| {
        record_analysis(
            transaction,
            &AppendAnalysisRecord {
                asset_id,
                record: AnalysisRecord::new(AnalysisKind::Transcript, value, model, None, now),
            },
        )?;
        // Keep the FTS5 index aligned with the newest transcript evidence.
        echo_catalog::index_transcript(transaction, &asset_id.to_string(), &payload.text).map_err(
            |error| {
                CoreError::new(
                    CoreErrorKind::Other,
                    format!("cannot index transcript: {error}"),
                )
            },
        )
    })
}

/// Records Runtime-produced forced-alignment evidence.
///
/// # Errors
///
/// Returns a catalog failure when the evidence cannot be committed.
pub fn record_alignment(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    payload: &AlignmentPayload,
) -> Result<(), CoreError> {
    let value = serde_json::to_value(payload).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode alignment: {error}"),
        )
    })?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        });
    catalog
        .with_transaction(|transaction| {
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Alignment,
                        value,
                        ModelIdentity::new(
                            payload.runtime.job.physical_model.clone(),
                            payload.runtime.job.model_build.clone(),
                        ),
                        None,
                        now,
                    ),
                },
            )
        })
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests;
