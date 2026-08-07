//! Transcript analysis: runs the ASR worker as a subprocess and records the
//! result as evidence in the catalog.
//!
//! The worker contract is a Python script (`tools/asr/transcribe.py`) invoked
//! with a configured interpreter; the JSON schema below is owned here.

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use echo_catalog::{AppendAnalysisRecord, record_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

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

/// The ASR worker invocation contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscribeWorker {
    /// Interpreter that can import `mlx_audio` (the MLX venv's python).
    pub python: PathBuf,
    /// The worker script (`tools/asr/transcribe.py`).
    pub script: PathBuf,
}

/// Runs the ASR worker on `source` with `model_snapshot` and returns the
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
    let output_path = temporary_output_path();
    let output = Command::new(&worker.python)
        .arg(&worker.script)
        .arg("--model")
        .arg(model_snapshot)
        .arg("--audio")
        .arg(source)
        .arg("--out")
        .arg(&output_path)
        .output()
        .map_err(|error| {
            CoreError::new(
                CoreErrorKind::Other,
                format!(
                    "cannot start ASR worker {}: {error}",
                    worker.python.display()
                ),
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&output_path);
        return Err(CoreError::new(
            CoreErrorKind::Other,
            format!("ASR worker failed ({}): {}", output.status, stderr.trim()),
        ));
    }
    let bytes = std::fs::read(&output_path).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!(
                "cannot read ASR worker output {}: {error}",
                output_path.display()
            ),
        )
    })?;
    let _ = std::fs::remove_file(&output_path);
    serde_json::from_slice(&bytes).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot parse ASR worker output: {error}"),
        )
    })
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
    catalog
        .with_transaction(|transaction| {
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
            )
        })
        .map_err(CoreError::from)
}

fn temporary_output_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "echo-transcript-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ))
}
