//! Transcript analysis: submits an Infer-compatible `audio.transcribe`
//! request to the temporary direct MLX adapter and records the result as
//! evidence in the catalog.
//!
//! The adapter is deliberately not a scheduler. It preserves the product
//! request contract while Infer Build is not yet the execution owner.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use echo_catalog::{AppendAnalysisRecord, record_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

/// Stable Infer Runtime intent consumed by Echo's first real inference slice.
pub const TRANSCRIPTION_INTENT: &str = "audio.transcribe";

/// Product-level transcription request. These fields mirror Infer Runtime's
/// `TranscriptionRequest`; the direct adapter only adds local execution data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionIntent {
    pub model: String,
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub response_format: String,
    pub temperature: Option<f64>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for TranscriptionIntent {
    fn default() -> Self {
        Self {
            model: TRANSCRIPTION_INTENT.to_owned(),
            language: None,
            prompt: None,
            response_format: "verbose_json".to_owned(),
            temperature: None,
            metadata: BTreeMap::from([
                ("infer.policy".to_owned(), "local-first".to_owned()),
                ("infer.placement".to_owned(), "local_only".to_owned()),
                ("infer.offline_required".to_owned(), "true".to_owned()),
                ("infer.fallback".to_owned(), "none".to_owned()),
            ]),
        }
    }
}

/// One transcribed segment with timestamps in seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub text: String,
    pub start: f64,
    pub end: f64,
    /// Word-level timestamps when the model provides them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<Vec<TranscriptWord>>,
}

/// Word-level timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptWord {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// Canonical transcript payload produced by the worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptPayload {
    pub model: String,
    pub language: Option<String>,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
}

/// Temporary local execution adapter for the Infer-compatible request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscribeWorker {
    /// Interpreter that can import `mlx_audio` (the MLX venv's python).
    pub python: PathBuf,
    /// The JSON-lines adapter (`tools/inference/local_audio_worker.py`).
    pub script: PathBuf,
}

#[derive(Debug, Serialize)]
struct DirectTranscriptionRequest {
    request_id: String,
    operation: &'static str,
    intent: TranscriptionIntent,
    model: String,
    audio_path: String,
}

#[derive(Debug, Deserialize)]
struct DirectTranscriptionResponse {
    request_id: String,
    ok: bool,
    #[serde(default)]
    result: Option<TranscriptPayload>,
    #[serde(default)]
    error: Option<String>,
}

/// Submits the Infer-compatible intent to the direct adapter and returns the
/// parsed transcript.
///
/// # Errors
///
/// Returns [`CoreErrorKind::Other`] when the worker cannot be started or
/// fails, and when its output cannot be parsed.
pub fn run_transcribe(
    source: &Path,
    model_snapshot: &Path,
    worker: &TranscribeWorker,
) -> Result<TranscriptPayload, CoreError> {
    let request = direct_request(source, model_snapshot);
    let request_id = request.request_id.clone();
    let mut child = Command::new(&worker.python)
        .arg(&worker.script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            CoreError::new(
                CoreErrorKind::Other,
                format!(
                    "cannot start local inference worker {}: {error}",
                    worker.python.display()
                ),
            )
        })?;
    let mut stdin = child.stdin.take().ok_or_else(|| {
        CoreError::new(CoreErrorKind::Other, "local inference worker has no stdin")
    })?;
    serde_json::to_writer(&mut stdin, &request).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode transcription intent: {error}"),
        )
    })?;
    stdin.write_all(b"\n").map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot submit transcription intent: {error}"),
        )
    })?;
    drop(stdin);
    let output = child.wait_with_output().map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot wait for local inference worker: {error}"),
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CoreError::new(
            CoreErrorKind::Other,
            format!(
                "local inference worker failed ({}): {}",
                output.status,
                stderr.trim()
            ),
        ));
    }
    let response: DirectTranscriptionResponse =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            CoreError::new(
                CoreErrorKind::Other,
                format!("cannot parse local inference response: {error}"),
            )
        })?;
    if response.request_id != request_id {
        return Err(CoreError::new(
            CoreErrorKind::Other,
            format!(
                "local inference response id mismatch: expected {request_id}, got {}",
                response.request_id
            ),
        ));
    }
    if !response.ok {
        return Err(CoreError::new(
            CoreErrorKind::Other,
            response
                .error
                .unwrap_or_else(|| "local inference failed without an error".to_owned()),
        ));
    }
    response.result.ok_or_else(|| {
        CoreError::new(
            CoreErrorKind::Other,
            "local inference succeeded without a transcript result",
        )
    })
}

fn direct_request(source: &Path, model_snapshot: &Path) -> DirectTranscriptionRequest {
    DirectTranscriptionRequest {
        request_id: format!(
            "echo-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ),
        operation: "transcribe",
        intent: TranscriptionIntent::default(),
        model: model_snapshot.to_string_lossy().into_owned(),
        audio_path: source.to_string_lossy().into_owned(),
    }
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
                record: AnalysisRecord::new(
                    AnalysisKind::Transcript,
                    value,
                    ModelIdentity::new("mlx/qwen3-asr".to_owned(), model_version.to_owned()),
                    None,
                    now,
                ),
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

#[cfg(test)]
mod tests;
