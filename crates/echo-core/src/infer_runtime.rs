//! Echo product intents adapted to the official Infer Runtime SDK.
//!
//! The SDK owns Discovery, credential loading, Core/Capability negotiation,
//! loopback transport, redirects/proxies, common headers and error envelopes.
//! Echo owns product constraints, evidence validation and persistence shapes.

mod audio_embeddings;
mod audio_events;
mod embeddings;
mod responses;
pub(crate) mod speech;

pub use audio_embeddings::{
    AUDIO_EMBEDDING_INTENT, AUDIO_TEXT_QUERY_EMBEDDING_INTENT, AudioEmbeddingPayload,
    AudioTextQueryEmbeddingIntent,
};
pub use audio_events::{
    AUDIO_EVENT_DETECTION_INTENT, AudioAnalysisCoverage, AudioCoverageStatus, AudioEventDetection,
    AudioEventDetectionIntent, DetectedAudioEvent, SoundEventDetectionPolicy, SoundEventOntology,
    SoundEventProvenance, SoundEventSmoothingPolicy, SpeechPresence, SpeechPresenceStatus,
};
pub use embeddings::{
    TEXT_EMBEDDING_INTENT, TextEmbeddingIntent, TextEmbeddingPayload,
    TextEmbeddingProviderProvenance,
};
pub use responses::{
    CONTEXTUAL_INTENT, ContextualIntent, ContextualResponse, MAX_CONTEXTUAL_INPUT_BYTES,
};

use std::{
    collections::BTreeMap,
    fmt,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use infer_runtime_client::{
    AlignmentResponse as SdkAlignmentResponse, AudioEmbeddingResponse as SdkAudioEmbeddingResponse,
    AudioEventDetectionResponse as SdkAudioEventDetectionResponse, AudioTextEmbeddingRequest,
    Client, DiscoveryResolver, Error as SdkError, JobSnapshot as SdkJobSnapshot, ResponsesRequest,
    ResponsesResult, TextEmbeddingRequest, TextEmbeddingResponse, TranscriptionFormat,
    TranscriptionResponse as SdkTranscriptionResponse,
};
use serde::{Deserialize, Serialize};

use crate::{TranscriptPayload, TranscriptSegment};

pub const EXPECTED_CONTRACT_VERSION: &str = infer_runtime_client::CONSUMER_CORE;
pub const TRANSCRIPTION_INTENT: &str = "audio.transcribe";
pub const ALIGNMENT_INTENT: &str = "audio.align";
pub const MAX_AUDIO_UPLOAD_BYTES: u64 = 25 * 1024 * 1024;

const EXPECTED_APP_ID: &str = "echo";
const TRANSCRIPTION_CAPABILITY: &str = "infer.audio.transcription@20260814.1";
const ALIGNMENT_CAPABILITY: &str = "infer.audio.alignment@20260811.1";
pub(crate) const AUDIO_EVENT_DETECTION_CAPABILITY: &str = "infer.audio.event-detection@20260813.2";
pub(crate) const AUDIO_EMBEDDING_CAPABILITY: &str = "infer.audio.embedding@20260815.2";

pub(crate) fn is_current_contract_version(version: &str) -> bool {
    version == EXPECTED_CONTRACT_VERSION
}

/// Official SDK inputs supplied by the Echo product host.
#[derive(Clone, PartialEq, Eq)]
pub struct InferRuntimeConfig {
    /// Explicit development endpoint override. Empty means Infra Discovery.
    pub base_url: String,
    /// Echo's existing managed credential file. The SDK reads it only when an
    /// authenticated request is sent.
    pub credential_path: PathBuf,
}

impl fmt::Debug for InferRuntimeConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InferRuntimeConfig")
            .field("base_url", &self.base_url)
            .field("credential_path", &self.credential_path)
            .finish()
    }
}

/// Product-level transcription request mapped to the typed SDK call.
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

pub(crate) fn default_background_constraints() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("infer.policy".to_owned(), "local-first".to_owned()),
        ("infer.priority".to_owned(), "background".to_owned()),
        ("infer.placement".to_owned(), "local_only".to_owned()),
        ("infer.prefer".to_owned(), "local".to_owned()),
        ("infer.offline_required".to_owned(), "true".to_owned()),
        (
            "infer.capability_floor".to_owned(),
            "foundational".to_owned(),
        ),
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
    #[serde(alias = "consumer_contract_version")]
    pub consumer_core_contract: String,
    #[serde(default)]
    pub capability_contract: Option<String>,
    pub app_id: String,
    pub intent: String,
    pub provider: String,
    pub deployment: String,
    pub model_profile: String,
    pub model_build: String,
    pub physical_model: String,
    pub placement: String,
    #[serde(default)]
    pub capability_level: String,
    #[serde(default)]
    pub evaluation_status: String,
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
    pub capability_floor: Option<String>,
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
    pub capability_floor: String,
    #[serde(default)]
    pub candidates: Vec<RuntimeCandidateDecision>,
}

/// One sanitized Runtime routing candidate. This is execution evidence, not a
/// legacy candidate protocol branch.
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

/// Sanitized Attempt machine fields. Provider raw error text is omitted.
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

/// Runtime failure without secrets, payloads or provider diagnostic text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Infer Runtime {code} ({kind:?}{status_suffix})")]
pub struct InferRuntimeError {
    pub kind: InferRuntimeErrorKind,
    pub code: String,
    pub http_status: Option<u16>,
    status_suffix: String,
}

impl InferRuntimeError {
    pub(crate) fn new(
        kind: InferRuntimeErrorKind,
        code: impl Into<String>,
        http_status: Option<u16>,
    ) -> Self {
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

/// Blocking adapter used only from Echo background worker threads.
#[derive(Debug, Clone)]
pub struct InferRuntimeClient {
    transport: Result<Arc<dyn RuntimeTransport>, InferRuntimeError>,
}

impl InferRuntimeClient {
    #[must_use]
    pub fn new(config: InferRuntimeConfig) -> Self {
        Self {
            transport: build_sdk_client(config)
                .map(SdkTransport::new)
                .map(|transport| Arc::new(transport) as Arc<dyn RuntimeTransport>),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_transport(transport: Arc<dyn RuntimeTransport>) -> Self {
        Self {
            transport: Ok(transport),
        }
    }

    /// Verifies the frozen dated Core contract through the SDK.
    ///
    /// # Errors
    ///
    /// Returns a classified Discovery, transport, schema or contract failure.
    pub fn contract_version(&self) -> Result<String, InferRuntimeError> {
        self.transport()?.contract()?;
        Ok(EXPECTED_CONTRACT_VERSION.to_owned())
    }

    /// Submits `audio.transcribe` and accepts only an Echo-scoped succeeded Job.
    ///
    /// # Errors
    ///
    /// Returns a bounded-input, SDK, Runtime or provenance validation failure.
    pub fn transcribe(
        &self,
        source: &Path,
        intent: &TranscriptionIntent,
    ) -> Result<TranscriptPayload, InferRuntimeError> {
        validate_source(source)?;
        if intent.model != TRANSCRIPTION_INTENT
            || intent.response_format != "verbose_json"
            || intent.prompt.is_some()
            || intent.temperature.is_some()
        {
            return Err(rejected("unsupported_transcription_options"));
        }
        let (response, job) =
            self.transport()?
                .transcribe(source, intent.language.as_deref(), &intent.metadata)?;
        let job = validate_succeeded_job(job, TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY)?;
        validate_local_only_job(&job, "inconsistent_transcription_constraints")?;
        let segments = response
            .segments
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| serde_json::from_value::<TranscriptSegment>(item.clone()).ok())
            .collect();
        Ok(TranscriptPayload {
            model: TRANSCRIPTION_INTENT.to_owned(),
            language: response.language,
            text: response.text,
            segments,
            runtime: Some(RuntimeProvenance {
                contract_version: EXPECTED_CONTRACT_VERSION.to_owned(),
                job,
            }),
        })
    }

    /// Submits `audio.align` and accepts only an Echo-scoped succeeded Job.
    ///
    /// # Errors
    ///
    /// Returns a bounded-input, SDK, Runtime or provenance validation failure.
    pub fn align(
        &self,
        source: &Path,
        text: &str,
        intent: &AlignmentIntent,
    ) -> Result<AlignmentPayload, InferRuntimeError> {
        validate_source(source)?;
        if intent.model != ALIGNMENT_INTENT || text.trim().is_empty() {
            return Err(rejected("alignment_text_required"));
        }
        let (response, job) =
            self.transport()?
                .align(source, text, intent.language.as_deref(), &intent.metadata)?;
        let job = validate_succeeded_job(job, ALIGNMENT_INTENT, ALIGNMENT_CAPABILITY)?;
        validate_local_only_job(&job, "inconsistent_alignment_constraints")?;
        Ok(AlignmentPayload {
            text: response.text,
            language: (!response.language.is_empty()).then_some(response.language),
            items: response
                .items
                .into_iter()
                .map(|item| AlignmentItem {
                    text: item.text,
                    start: item.start,
                    end: item.end,
                })
                .collect(),
            runtime: RuntimeProvenance {
                contract_version: EXPECTED_CONTRACT_VERSION.to_owned(),
                job,
            },
        })
    }

    pub(crate) fn transport(&self) -> Result<Arc<dyn RuntimeTransport>, InferRuntimeError> {
        self.transport.clone()
    }
}

pub(crate) trait RuntimeTransport: fmt::Debug + Send + Sync {
    fn contract(&self) -> Result<(), InferRuntimeError>;

    fn transcribe(
        &self,
        source: &Path,
        language: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkTranscriptionResponse, SdkJobSnapshot), InferRuntimeError>;

    fn align(
        &self,
        source: &Path,
        text: &str,
        language: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAlignmentResponse, SdkJobSnapshot), InferRuntimeError>;

    fn detect_audio_events(
        &self,
        source: &Path,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAudioEventDetectionResponse, SdkJobSnapshot), InferRuntimeError>;

    fn contextualize(
        &self,
        request: &ResponsesRequest,
    ) -> Result<(ResponsesResult, SdkJobSnapshot), InferRuntimeError>;

    fn embed_text(
        &self,
        request: &TextEmbeddingRequest,
    ) -> Result<(TextEmbeddingResponse, SdkJobSnapshot), InferRuntimeError>;

    fn embed_audio(
        &self,
        source: &Path,
        source_revision: &str,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAudioEmbeddingResponse, SdkJobSnapshot), InferRuntimeError>;

    fn embed_audio_text(
        &self,
        request: &AudioTextEmbeddingRequest,
    ) -> Result<(SdkAudioEmbeddingResponse, SdkJobSnapshot), InferRuntimeError>;
}

#[derive(Debug)]
struct SdkTransport {
    client: Client,
}

impl SdkTransport {
    const fn new(client: Client) -> Self {
        Self { client }
    }

    fn run<T>(
        future: impl Future<Output = infer_runtime_client::Result<T>>,
    ) -> Result<T, InferRuntimeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| protocol("runtime_executor_unavailable"))?;
        runtime.block_on(future).map_err(map_sdk_error)
    }
}

impl RuntimeTransport for SdkTransport {
    fn contract(&self) -> Result<(), InferRuntimeError> {
        Self::run(async { self.client.contract().await }).map(|_| ())
    }

    fn transcribe(
        &self,
        source: &Path,
        language: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkTranscriptionResponse, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self
                .client
                .transcribe_file(
                    source,
                    audio_content_type(source),
                    language,
                    TranscriptionFormat::VerboseJson,
                    metadata,
                )
                .await?;
            let response_id = response_id(&response.extra)?;
            let job = self.client.job(response_id).await?;
            Ok((response, job))
        })
    }

    fn align(
        &self,
        source: &Path,
        text: &str,
        language: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAlignmentResponse, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self
                .client
                .align_file(source, audio_content_type(source), text, language, metadata)
                .await?;
            let response_id = response_id(&response.extra)?;
            let job = self.client.job(response_id).await?;
            Ok((response, job))
        })
    }

    fn detect_audio_events(
        &self,
        source: &Path,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAudioEventDetectionResponse, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self
                .client
                .detect_audio_events_file(source, audio_content_type(source), metadata)
                .await?;
            let job = self.client.job(&response.id).await?;
            Ok((response, job))
        })
    }

    fn contextualize(
        &self,
        request: &ResponsesRequest,
    ) -> Result<(ResponsesResult, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self.client.create_response(request).await?;
            let job = self.client.job(&response.id).await?;
            Ok((response, job))
        })
    }

    fn embed_text(
        &self,
        request: &TextEmbeddingRequest,
    ) -> Result<(TextEmbeddingResponse, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self.client.embed_text(request).await?;
            let job = self.client.job(&response.id).await?;
            Ok((response, job))
        })
    }

    fn embed_audio(
        &self,
        source: &Path,
        source_revision: &str,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAudioEmbeddingResponse, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self
                .client
                .embed_audio_file(
                    source,
                    audio_content_type(source),
                    source_revision,
                    metadata,
                )
                .await?;
            let job = self.client.job(&response.id).await?;
            Ok((response, job))
        })
    }

    fn embed_audio_text(
        &self,
        request: &AudioTextEmbeddingRequest,
    ) -> Result<(SdkAudioEmbeddingResponse, SdkJobSnapshot), InferRuntimeError> {
        Self::run(async {
            let response = self.client.embed_audio_text(request).await?;
            let job = self.client.job(&response.id).await?;
            Ok((response, job))
        })
    }
}

fn response_id(extra: &BTreeMap<String, serde_json::Value>) -> infer_runtime_client::Result<&str> {
    extra
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| SdkError::MalformedResponse("missing response id".into()))
}

fn build_sdk_client(config: InferRuntimeConfig) -> Result<Client, InferRuntimeError> {
    let resolver = if config.base_url.trim().is_empty() {
        DiscoveryResolver::local()
    } else {
        DiscoveryResolver::local()
            .with_explicit_endpoint(config.base_url)
            .map_err(|_| protocol("invalid_runtime_endpoint"))?
    };
    Client::with_discovery(resolver)
        .credential_file(config.credential_path)
        .build()
        .map_err(map_sdk_error)
}

fn validate_source(source: &Path) -> Result<(), InferRuntimeError> {
    let size = std::fs::metadata(source)
        .map_err(|_| protocol("source_unavailable"))?
        .len();
    if size == 0 {
        return Err(rejected("empty_audio"));
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

fn audio_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav" | "wave") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("m4a" | "mp4") => "audio/mp4",
        Some("ogg" | "opus") => "audio/ogg",
        _ => "application/octet-stream",
    }
}

pub(crate) fn validate_succeeded_job(
    job: SdkJobSnapshot,
    expected_intent: &str,
    expected_capability: &str,
) -> Result<RuntimeJobSnapshot, InferRuntimeError> {
    if job.app_id != EXPECTED_APP_ID
        || job.intent != expected_intent
        || job.state != "succeeded"
        || job.consumer_core_contract != EXPECTED_CONTRACT_VERSION
        || job.capability_contract.as_deref() != Some(expected_capability)
    {
        return Err(protocol("inconsistent_job_snapshot"));
    }
    let constraints =
        serde_json::from_value(job.constraints).map_err(|_| protocol("invalid_job_constraints"))?;
    let attempts = job
        .attempts
        .into_iter()
        .map(|attempt| {
            Ok(RuntimeAttempt {
                number: u32::try_from(attempt.number)
                    .map_err(|_| protocol("invalid_job_attempt"))?,
                provider: attempt.provider,
                deployment: attempt.deployment,
                outcome: attempt.outcome,
                trigger: attempt.trigger,
                error_kind: attempt.error_kind,
            })
        })
        .collect::<Result<Vec<_>, InferRuntimeError>>()?;
    Ok(RuntimeJobSnapshot {
        id: job.id,
        consumer_core_contract: job.consumer_core_contract,
        capability_contract: job.capability_contract,
        app_id: job.app_id,
        intent: job.intent,
        provider: job.provider,
        deployment: job.deployment,
        model_profile: job.model_profile,
        model_build: job.model_build,
        physical_model: job.physical_model,
        placement: job.placement,
        capability_level: job.capability_level,
        evaluation_status: job.evaluation_status,
        resource_class: job.resource_class,
        state: job.state,
        policy: job.policy,
        priority: job.priority,
        constraints,
        routing: RuntimeRoutingDecision {
            capability_floor: job.routing.capability_floor,
            candidates: job
                .routing
                .candidates
                .into_iter()
                .map(|candidate| RuntimeCandidateDecision {
                    deployment: candidate.deployment,
                    provider: candidate.provider,
                    status: candidate.status,
                    rank: candidate.rank,
                    reason_codes: candidate.reason_codes,
                })
                .collect(),
        },
        attempts,
    })
}

pub(crate) fn validate_local_only_job(
    job: &RuntimeJobSnapshot,
    code: &'static str,
) -> Result<(), InferRuntimeError> {
    let constraints = &job.constraints;
    let valid = job.policy == "local-first"
        && job.priority == "background"
        && job.placement == "local"
        && capability_level_meets_foundational_floor(&job.capability_level)
        && constraints.policy.as_deref() == Some("local-first")
        && constraints.priority.as_deref() == Some("background")
        && constraints.placement.as_deref() == Some("local_only")
        && constraints.prefer.as_deref() == Some("local")
        && constraints.offline_required == Some(true)
        && constraints.capability_floor.as_deref() == Some("foundational")
        && constraints.latency.as_deref() == Some("throughput")
        && constraints.max_cost_usd == Some(0.0)
        && constraints.fallback.as_deref() == Some("none")
        && job.routing.capability_floor == "foundational"
        && job
            .attempts
            .iter()
            .all(|attempt| attempt.trigger != "fallback");
    if valid { Ok(()) } else { Err(protocol(code)) }
}

fn capability_level_meets_foundational_floor(level: &str) -> bool {
    matches!(
        level,
        "foundational" | "capable" | "advanced" | "expert" | "exceptional"
    )
}

fn map_sdk_error(error: SdkError) -> InferRuntimeError {
    match error {
        SdkError::Discovery(_) => InferRuntimeError::new(
            InferRuntimeErrorKind::Unavailable,
            "runtime_discovery_unavailable",
            None,
        ),
        SdkError::Credential(_) => InferRuntimeError::new(
            InferRuntimeErrorKind::Authentication,
            "runtime_credential_unavailable",
            None,
        ),
        SdkError::Input(_) => rejected("invalid_runtime_input"),
        SdkError::Transport(error) if error.is_timeout() => {
            InferRuntimeError::new(InferRuntimeErrorKind::Deadline, "deadline_exceeded", None)
        }
        SdkError::Transport(_) => InferRuntimeError::new(
            InferRuntimeErrorKind::Unavailable,
            "runtime_unavailable",
            None,
        ),
        SdkError::ContractMismatch => InferRuntimeError::new(
            InferRuntimeErrorKind::ContractMismatch,
            "consumer_core_unsupported",
            None,
        ),
        SdkError::Api { status, code, .. } => InferRuntimeError::new(
            classify_status_and_code(status.as_u16(), &code),
            code,
            Some(status.as_u16()),
        ),
        SdkError::MalformedResponse(_) => protocol("invalid_runtime_response"),
    }
}

fn classify_status_and_code(status: u16, code: &str) -> InferRuntimeErrorKind {
    match code {
        "consumer_core_unsupported" | "capability_contract_unsupported" => {
            InferRuntimeErrorKind::ContractMismatch
        }
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

pub(crate) fn rejected(code: &'static str) -> InferRuntimeError {
    InferRuntimeError::new(InferRuntimeErrorKind::Rejected, code, None)
}

pub(crate) fn protocol(code: &'static str) -> InferRuntimeError {
    InferRuntimeError::new(InferRuntimeErrorKind::Protocol, code, Some(200))
}

#[cfg(test)]
pub(crate) mod tests;
