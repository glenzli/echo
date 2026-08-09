//! Versioned Infer Runtime consumer for Echo's audio understanding intents.
//!
//! This owner contains the external wire contract: contract discovery,
//! authentication, bounded streaming multipart, stable error classification,
//! response normalization and App-scoped Job provenance. It never opens the
//! catalog and never knows how Echo schedules or presents analysis work.

mod responses;

pub use responses::{
    CONTEXTUAL_INTENT, ContextualIntent, ContextualResponse, MAX_CONTEXTUAL_INPUT_BYTES,
};

use std::{collections::BTreeMap, fmt, path::Path, time::Duration};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use ureq::unversioned::multipart::Form;

use crate::TranscriptPayload;

pub const EXPECTED_CONTRACT_VERSION: &str = "0.1.0-candidate.1";
pub const TRANSCRIPTION_INTENT: &str = "audio.transcribe";
pub const ALIGNMENT_INTENT: &str = "audio.align";
pub const MAX_AUDIO_UPLOAD_BYTES: u64 = 25 * 1024 * 1024;

const MAX_JSON_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const EXPECTED_APP_ID: &str = "echo";

/// Runtime endpoint and secret injected by the product host.
#[derive(Clone, PartialEq, Eq)]
pub struct InferRuntimeConfig {
    pub base_url: String,
    pub bearer_token: String,
}

impl fmt::Debug for InferRuntimeConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InferRuntimeConfig")
            .field("base_url", &self.base_url)
            .field("bearer_token", &"[redacted]")
            .finish()
    }
}

/// Product-level transcription request mapped to Runtime multipart fields.
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
            metadata: default_background_constraints(),
        }
    }
}

/// Product-level forced-alignment request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignmentIntent {
    pub model: String,
    pub language: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for AlignmentIntent {
    fn default() -> Self {
        Self {
            model: ALIGNMENT_INTENT.to_owned(),
            language: None,
            metadata: default_background_constraints(),
        }
    }
}

fn default_background_constraints() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("infer.policy".to_owned(), "local-first".to_owned()),
        ("infer.priority".to_owned(), "background".to_owned()),
        ("infer.placement".to_owned(), "local_only".to_owned()),
        ("infer.prefer".to_owned(), "local".to_owned()),
        ("infer.offline_required".to_owned(), "true".to_owned()),
        ("infer.quality_floor".to_owned(), "basic".to_owned()),
        ("infer.latency".to_owned(), "throughput".to_owned()),
        ("infer.fallback".to_owned(), "none".to_owned()),
        ("infer.max_cost_usd".to_owned(), "0".to_owned()),
    ])
}

/// Word/unit timing returned by forced alignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentItem {
    pub text: String,
    #[serde(alias = "start_time")]
    pub start: f64,
    #[serde(alias = "end_time")]
    pub end: f64,
}

/// Canonical alignment evidence plus Runtime provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentPayload {
    pub text: String,
    pub language: Option<String>,
    #[serde(default)]
    pub items: Vec<AlignmentItem>,
    pub runtime: RuntimeProvenance,
}

/// Sanitized Runtime execution provenance retained with Echo evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeProvenance {
    pub contract_version: String,
    pub job: RuntimeJobSnapshot,
}

/// App-scoped Job fields Echo can safely persist and diagnose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeJobSnapshot {
    pub id: String,
    pub app_id: String,
    pub intent: String,
    pub provider: String,
    pub deployment: String,
    pub model_profile: String,
    pub model_build: String,
    pub physical_model: String,
    pub placement: String,
    #[serde(default)]
    pub quality_grade: String,
    #[serde(default)]
    pub rating_status: String,
    #[serde(default)]
    pub resource_class: String,
    pub state: String,
    pub policy: String,
    pub priority: String,
    #[serde(default)]
    pub constraints: RuntimeJobConstraints,
    #[serde(default)]
    pub routing: RuntimeRoutingDecision,
    #[serde(default)]
    pub attempts: Vec<RuntimeAttempt>,
}

/// Sanitized admission constraints retained with each Runtime result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuntimeJobConstraints {
    #[serde(default)]
    pub policy: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub placement: Option<String>,
    #[serde(default)]
    pub prefer: Option<String>,
    #[serde(default)]
    pub offline_required: Option<bool>,
    #[serde(default)]
    pub quality_floor: Option<String>,
    #[serde(default)]
    pub latency: Option<String>,
    #[serde(default)]
    pub max_cost_usd: Option<f64>,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default)]
    pub deadline_ms: Option<u64>,
}

/// Immutable admission-time routing evidence retained without provider errors.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRoutingDecision {
    #[serde(default)]
    pub quality_floor: String,
    #[serde(default)]
    pub candidates: Vec<RuntimeCandidateDecision>,
}

/// One sanitized Runtime routing candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCandidateDecision {
    pub deployment: String,
    pub provider: String,
    pub status: String,
    #[serde(default)]
    pub rank: Option<usize>,
    #[serde(default)]
    pub reason_codes: Vec<String>,
}

/// Sanitized Attempt machine fields. Provider raw error text is intentionally
/// omitted even when Runtime adds it to the response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeAttempt {
    pub number: u32,
    pub provider: String,
    pub deployment: String,
    pub outcome: String,
    pub trigger: String,
    #[serde(default)]
    pub error_kind: Option<String>,
}

/// Stable consumer-side failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferRuntimeErrorKind {
    Unavailable,
    ContractMismatch,
    Authentication,
    Rejected,
    Capacity,
    Cancelled,
    Deadline,
    Protocol,
    SourceTooLarge,
}

/// Runtime failure without secrets or provider diagnostic text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Infer Runtime {code} ({kind:?}{status_suffix})")]
pub struct InferRuntimeError {
    pub kind: InferRuntimeErrorKind,
    pub code: String,
    pub http_status: Option<u16>,
    status_suffix: String,
}

impl InferRuntimeError {
    fn new(kind: InferRuntimeErrorKind, code: impl Into<String>, http_status: Option<u16>) -> Self {
        Self {
            kind,
            code: code.into(),
            http_status,
            status_suffix: http_status
                .map_or_else(String::new, |status| format!(", HTTP {status}")),
        }
    }

    /// Whether retry can succeed without changing the source or request.
    #[must_use]
    pub fn retryable(&self) -> bool {
        matches!(
            self.kind,
            InferRuntimeErrorKind::Unavailable
                | InferRuntimeErrorKind::Capacity
                | InferRuntimeErrorKind::Deadline
        )
    }
}

/// Blocking consumer used only from Echo's background worker threads.
#[derive(Debug, Clone)]
pub struct InferRuntimeClient {
    config: InferRuntimeConfig,
}

impl InferRuntimeClient {
    #[must_use]
    pub const fn new(config: InferRuntimeConfig) -> Self {
        Self { config }
    }

    /// Discovers and verifies the immutable candidate contract.
    ///
    /// # Errors
    ///
    /// Returns a classified transport, protocol, or version mismatch.
    pub fn contract_version(&self) -> Result<String, InferRuntimeError> {
        let url = self.url("/infer/v1/contract")?;
        let response = ureq::get(&url)
            .config()
            .timeout_global(Some(Duration::from_secs(3)))
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(transport_error)?;
        let (_, body) = checked_json_response(response)?;
        let manifest: ContractManifest = serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_contract",
                Some(200),
            )
        })?;
        if manifest.contract_version != EXPECTED_CONTRACT_VERSION {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::ContractMismatch,
                "contract_mismatch",
                Some(200),
            ));
        }
        Ok(manifest.contract_version)
    }

    /// Submits `audio.transcribe`, then requires its App-scoped Job snapshot.
    ///
    /// # Errors
    ///
    /// Returns a bounded source, transport, Runtime, response, or provenance failure.
    pub fn transcribe(
        &self,
        source: &Path,
        intent: &TranscriptionIntent,
    ) -> Result<TranscriptPayload, InferRuntimeError> {
        Self::validate_source(source)?;
        self.validate_token()?;
        let contract_version = self.contract_version()?;
        let metadata = encode_metadata(&intent.metadata)?;
        let temperature = intent.temperature.map(|value| value.to_string());
        let mut form = Form::new()
            .text("model", &intent.model)
            .text("response_format", &intent.response_format)
            .text("metadata", &metadata)
            .file("file", source)
            .map_err(|_| {
                InferRuntimeError::new(InferRuntimeErrorKind::Protocol, "source_unavailable", None)
            })?;
        if let Some(language) = &intent.language {
            form = form.text("language", language);
        }
        if let Some(prompt) = &intent.prompt {
            form = form.text("prompt", prompt);
        }
        if let Some(temperature) = &temperature {
            form = form.text("temperature", temperature);
        }
        let body = self.post_form("/v1/audio/transcriptions", form)?;
        let response: AudioJsonResponse = serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_transcription_response",
                Some(200),
            )
        })?;
        if response.model != TRANSCRIPTION_INTENT || response.id.is_empty() {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_transcription_identity",
                Some(200),
            ));
        }
        let job = self.job_snapshot(&response.id)?;
        validate_succeeded_job(&job, TRANSCRIPTION_INTENT)?;
        Ok(TranscriptPayload {
            model: response.model,
            language: extension_string(&response.extensions, "language"),
            text: extension_string(&response.extensions, "text").unwrap_or_default(),
            segments: extension_items(&response.extensions, "segments"),
            runtime: Some(RuntimeProvenance {
                contract_version,
                job,
            }),
        })
    }

    /// Submits `audio.align`, then requires its App-scoped Job snapshot.
    ///
    /// # Errors
    ///
    /// Returns a bounded source, transport, Runtime, response, or provenance failure.
    pub fn align(
        &self,
        source: &Path,
        text: &str,
        intent: &AlignmentIntent,
    ) -> Result<AlignmentPayload, InferRuntimeError> {
        Self::validate_source(source)?;
        self.validate_token()?;
        if text.trim().is_empty() {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Rejected,
                "alignment_text_required",
                None,
            ));
        }
        let contract_version = self.contract_version()?;
        let metadata = encode_metadata(&intent.metadata)?;
        let mut form = Form::new()
            .text("model", &intent.model)
            .text("text", text)
            .text("metadata", &metadata)
            .file("file", source)
            .map_err(|_| {
                InferRuntimeError::new(InferRuntimeErrorKind::Protocol, "source_unavailable", None)
            })?;
        if let Some(language) = &intent.language {
            form = form.text("language", language);
        }
        let body = self.post_form("/v1/audio/alignments", form)?;
        let response: AudioJsonResponse = serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_alignment_response",
                Some(200),
            )
        })?;
        if response.model != ALIGNMENT_INTENT || response.id.is_empty() {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_alignment_identity",
                Some(200),
            ));
        }
        let job = self.job_snapshot(&response.id)?;
        validate_succeeded_job(&job, ALIGNMENT_INTENT)?;
        Ok(AlignmentPayload {
            text: extension_string(&response.extensions, "text").unwrap_or_default(),
            language: extension_string(&response.extensions, "language"),
            items: extension_items(&response.extensions, "items"),
            runtime: RuntimeProvenance {
                contract_version,
                job,
            },
        })
    }

    fn job_snapshot(&self, response_id: &str) -> Result<RuntimeJobSnapshot, InferRuntimeError> {
        let path = format!("/infer/v1/jobs/{response_id}");
        let url = self.url(&path)?;
        let response = ureq::get(&url)
            .header("Authorization", self.authorization())
            .config()
            .timeout_global(Some(Duration::from_secs(5)))
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(transport_error)?;
        let (_, body) = checked_json_response(response)?;
        serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_job_snapshot",
                Some(200),
            )
        })
    }

    fn post_form(&self, path: &str, form: Form<'_>) -> Result<String, InferRuntimeError> {
        let url = self.url(path)?;
        let response = ureq::post(&url)
            .header("Authorization", self.authorization())
            .config()
            .timeout_global(Some(Duration::from_mins(30)))
            .http_status_as_error(false)
            .build()
            .send(form)
            .map_err(transport_error)?;
        checked_json_response(response).map(|(_, body)| body)
    }

    fn validate_source(source: &Path) -> Result<(), InferRuntimeError> {
        let size = std::fs::metadata(source)
            .map_err(|_| {
                InferRuntimeError::new(InferRuntimeErrorKind::Protocol, "source_unavailable", None)
            })?
            .len();
        if size == 0 {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Rejected,
                "empty_audio",
                None,
            ));
        }
        if size > MAX_AUDIO_UPLOAD_BYTES {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::SourceTooLarge,
                "audio_file_too_large",
                None,
            ));
        }
        Ok(())
    }

    fn validate_token(&self) -> Result<(), InferRuntimeError> {
        if self.config.bearer_token.trim().is_empty() {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Authentication,
                "missing_api_key",
                None,
            ));
        }
        Ok(())
    }

    fn authorization(&self) -> String {
        format!("Bearer {}", self.config.bearer_token)
    }

    fn url(&self, path: &str) -> Result<String, InferRuntimeError> {
        let base = self.config.base_url.trim().trim_end_matches('/');
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_runtime_endpoint",
                None,
            ));
        }
        Ok(format!("{base}{path}"))
    }
}

fn encode_metadata(metadata: &BTreeMap<String, String>) -> Result<String, InferRuntimeError> {
    serde_json::to_string(metadata).map_err(|_| {
        InferRuntimeError::new(
            InferRuntimeErrorKind::Protocol,
            "invalid_runtime_metadata",
            None,
        )
    })
}

fn checked_json_response(
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<(u16, String), InferRuntimeError> {
    let status = response.status().as_u16();
    let is_json = response
        .headers()
        .get(ureq::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .split(';')
        .next()
        .is_some_and(|media_type| media_type.trim() == "application/json");
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_JSON_RESPONSE_BYTES)
        .read_to_string()
        .map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_runtime_body",
                Some(status),
            )
        })?;
    if (200..300).contains(&status) {
        if !is_json {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "unexpected_runtime_media_type",
                Some(status),
            ));
        }
        return Ok((status, body));
    }
    let code = serde_json::from_str::<ErrorEnvelope>(&body).map_or_else(
        |_| status_fallback_code(status).to_owned(),
        |error| error.error.code,
    );
    Err(InferRuntimeError::new(
        classify_status_and_code(status, &code),
        code,
        Some(status),
    ))
}

fn classify_status_and_code(status: u16, code: &str) -> InferRuntimeErrorKind {
    match code {
        "invalid_api_key" | "missing_api_key" => InferRuntimeErrorKind::Authentication,
        "cancelled" => InferRuntimeErrorKind::Cancelled,
        "deadline_exceeded" => InferRuntimeErrorKind::Deadline,
        "queue_full" | "app_queue_full" | "quota_exceeded" | "provider_unavailable" => {
            InferRuntimeErrorKind::Capacity
        }
        _ if matches!(status, 429 | 503) => InferRuntimeErrorKind::Capacity,
        _ if status == 504 => InferRuntimeErrorKind::Deadline,
        _ => InferRuntimeErrorKind::Rejected,
    }
}

fn status_fallback_code(status: u16) -> &'static str {
    match status {
        400 => "invalid_request_error",
        401 => "invalid_api_key",
        404 => "not_found",
        409 => "conflict",
        429 => "queue_full",
        503 => "provider_unavailable",
        504 => "deadline_exceeded",
        _ => "runtime_http_error",
    }
}

fn transport_error(_: ureq::Error) -> InferRuntimeError {
    InferRuntimeError::new(
        InferRuntimeErrorKind::Unavailable,
        "runtime_unavailable",
        None,
    )
}

fn validate_succeeded_job(
    job: &RuntimeJobSnapshot,
    expected_intent: &str,
) -> Result<(), InferRuntimeError> {
    if job.app_id != EXPECTED_APP_ID || job.intent != expected_intent || job.state != "succeeded" {
        return Err(InferRuntimeError::new(
            InferRuntimeErrorKind::Protocol,
            "inconsistent_job_snapshot",
            Some(200),
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ContractManifest {
    contract_version: String,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    code: String,
}

#[derive(Debug, Deserialize)]
struct AudioJsonResponse {
    id: String,
    model: String,
    #[serde(flatten)]
    extensions: BTreeMap<String, Value>,
}

fn extension_string(extensions: &BTreeMap<String, Value>, field: &str) -> Option<String> {
    extensions
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn extension_items<T: DeserializeOwned>(
    extensions: &BTreeMap<String, Value>,
    field: &str,
) -> Vec<T> {
    extensions
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| serde_json::from_value(item.clone()).ok())
        .collect()
}

#[cfg(test)]
mod tests;
