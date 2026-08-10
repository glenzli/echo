//! Infer Runtime consumer for temporal, multi-label sound-event evidence.
//!
//! Runtime owns decoding, `YAMNet` execution and the versioned threshold policy.
//! Echo accepts only the stable task contract and sanitized App-scoped Job
//! provenance; product presentation remains outside this transport owner.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ureq::unversioned::multipart::Form;

use super::{
    InferRuntimeClient, InferRuntimeError, InferRuntimeErrorKind, RuntimeJobSnapshot,
    RuntimeProvenance, default_background_constraints, encode_metadata, validate_succeeded_job,
};

/// Stable Runtime Intent for bounded sound-event detection.
pub const AUDIO_EVENT_DETECTION_INTENT: &str = "audio.detect_events";
const AUDIO_EVENT_DETECTION_OBJECT: &str = "audio.event_detection";

/// Product-level request mapped to Runtime's strict multipart fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioEventDetectionIntent {
    pub model: String,
    pub metadata: BTreeMap<String, String>,
}

impl Default for AudioEventDetectionIntent {
    fn default() -> Self {
        Self {
            model: AUDIO_EVENT_DETECTION_INTENT.to_owned(),
            metadata: default_background_constraints(),
        }
    }
}

/// One temporal `AudioSet` class observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectedAudioEvent {
    /// Stable `AudioSet` MID. `label` is presentation text, not identity.
    pub class_id: String,
    pub label: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub score: f64,
}

/// Runtime's calibrated speech-presence state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeechPresenceStatus {
    Present,
    Absent,
    Unknown,
}

/// Speech-family evidence derived by the versioned Runtime policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechPresence {
    pub status: SpeechPresenceStatus,
    pub max_score: f64,
}

/// Coverage state for one bounded source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCoverageStatus {
    Full,
    Partial,
    None,
}

/// Exact input coverage retained with every event result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioAnalysisCoverage {
    pub status: AudioCoverageStatus,
    pub input_duration_seconds: f64,
    pub analyzed_start_seconds: f64,
    pub analyzed_end_seconds: f64,
    pub analyzed_seconds: f64,
    pub ratio: f64,
    pub window_count: usize,
    pub window_seconds: f64,
    pub hop_seconds: f64,
}

/// Stable ontology identity for `class_id` values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventOntology {
    pub id: String,
    pub revision: String,
    pub class_id_namespace: String,
    pub class_count: usize,
    pub artifact_sha256: String,
    pub license_spdx: String,
}

/// Temporal smoothing declared by Runtime's detection policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventSmoothingPolicy {
    pub method: String,
    pub window_frames: usize,
}

/// Versioned score interpretation and thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventDetectionPolicy {
    pub revision: String,
    pub score_kind: String,
    pub event_score_threshold: f64,
    pub smoothing: SoundEventSmoothingPolicy,
    pub max_classes_per_window: usize,
    pub speech_present_threshold: f64,
    pub speech_absent_threshold: f64,
}

/// Physical model, artifacts and preprocessing used by the provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventProvenance {
    pub model: String,
    pub model_archive_sha256: String,
    pub model_license_spdx: String,
    pub training_data_license_spdx: String,
    pub runtime: String,
    pub runtime_version: String,
    pub decoder: String,
    pub decoder_version: String,
    pub preprocessing_identity: String,
}

/// Accepted Runtime result plus the App-scoped Job that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioEventDetection {
    pub id: String,
    pub model: String,
    pub object: String,
    pub events: Vec<DetectedAudioEvent>,
    pub speech_presence: SpeechPresence,
    pub coverage: AudioAnalysisCoverage,
    pub ontology: SoundEventOntology,
    pub policy: SoundEventDetectionPolicy,
    pub provenance: SoundEventProvenance,
    pub runtime: RuntimeProvenance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireAudioEventDetection {
    id: String,
    model: String,
    object: String,
    events: Vec<DetectedAudioEvent>,
    speech_presence: SpeechPresence,
    coverage: AudioAnalysisCoverage,
    ontology: SoundEventOntology,
    policy: SoundEventDetectionPolicy,
    provenance: SoundEventProvenance,
}

impl InferRuntimeClient {
    /// Submits `audio.detect_events` and requires full contract and Job evidence.
    ///
    /// # Errors
    ///
    /// Returns a bounded source, transport, Runtime, response, policy, or
    /// provenance failure.
    pub fn detect_audio_events(
        &self,
        source: &std::path::Path,
        intent: &AudioEventDetectionIntent,
    ) -> Result<AudioEventDetection, InferRuntimeError> {
        Self::validate_source(source)?;
        self.validate_token()?;
        let contract_version = self.contract_version()?;
        let metadata = encode_metadata(&intent.metadata)?;
        let form = Form::new()
            .text("model", &intent.model)
            .text("metadata", &metadata)
            .file("file", source)
            .map_err(|_| {
                InferRuntimeError::new(InferRuntimeErrorKind::Protocol, "source_unavailable", None)
            })?;
        let body = self.post_form("/v1/audio/event-detections", form)?;
        let response: WireAudioEventDetection = serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_audio_event_response",
                Some(200),
            )
        })?;
        validate_detection(&response)?;
        let job = self.job_snapshot(&response.id)?;
        validate_succeeded_job(&job, AUDIO_EVENT_DETECTION_INTENT)?;
        validate_local_only_job(&job)?;
        Ok(AudioEventDetection {
            id: response.id,
            model: response.model,
            object: response.object,
            events: response.events,
            speech_presence: response.speech_presence,
            coverage: response.coverage,
            ontology: response.ontology,
            policy: response.policy,
            provenance: response.provenance,
            runtime: RuntimeProvenance {
                contract_version,
                job,
            },
        })
    }
}

fn validate_detection(response: &WireAudioEventDetection) -> Result<(), InferRuntimeError> {
    let identity_valid = !response.id.is_empty()
        && response.model == AUDIO_EVENT_DETECTION_INTENT
        && response.object == AUDIO_EVENT_DETECTION_OBJECT;
    if identity_valid
        && valid_contract_metadata(response)
        && valid_coverage(response)
        && valid_speech_state(response)
        && valid_events(response)
    {
        return Ok(());
    }
    Err(InferRuntimeError::new(
        InferRuntimeErrorKind::Protocol,
        "invalid_audio_event_contract",
        Some(200),
    ))
}

fn valid_contract_metadata(response: &WireAudioEventDetection) -> bool {
    let policy = &response.policy;
    !response.ontology.id.trim().is_empty()
        && !response.ontology.revision.trim().is_empty()
        && response.ontology.class_id_namespace == "audioset_mid"
        && response.ontology.class_count > 0
        && valid_sha256(&response.ontology.artifact_sha256)
        && !response.ontology.license_spdx.trim().is_empty()
        && !policy.revision.trim().is_empty()
        && policy.score_kind == "raw_sigmoid"
        && unit_interval(policy.event_score_threshold)
        && unit_interval(policy.speech_present_threshold)
        && unit_interval(policy.speech_absent_threshold)
        && policy.speech_absent_threshold < policy.speech_present_threshold
        && !policy.smoothing.method.trim().is_empty()
        && policy.smoothing.window_frames > 0
        && !policy.smoothing.window_frames.is_multiple_of(2)
        && policy.max_classes_per_window > 0
        && !response.provenance.model.trim().is_empty()
        && valid_sha256(&response.provenance.model_archive_sha256)
        && !response.provenance.model_license_spdx.trim().is_empty()
        && !response
            .provenance
            .training_data_license_spdx
            .trim()
            .is_empty()
        && !response.provenance.runtime.trim().is_empty()
        && !response.provenance.runtime_version.trim().is_empty()
        && !response.provenance.decoder.trim().is_empty()
        && !response.provenance.decoder_version.trim().is_empty()
        && !response.provenance.preprocessing_identity.trim().is_empty()
}

fn valid_coverage(response: &WireAudioEventDetection) -> bool {
    let coverage = &response.coverage;
    let numeric_valid = finite_positive(coverage.input_duration_seconds)
        && finite_non_negative(coverage.analyzed_start_seconds)
        && finite_non_negative(coverage.analyzed_end_seconds)
        && finite_non_negative(coverage.analyzed_seconds)
        && unit_interval(coverage.ratio)
        && finite_positive(coverage.window_seconds)
        && finite_positive(coverage.hop_seconds)
        && coverage.analyzed_start_seconds <= coverage.analyzed_end_seconds
        && coverage.analyzed_end_seconds <= coverage.input_duration_seconds + 1e-6
        && coverage.analyzed_seconds <= coverage.input_duration_seconds + 1e-6
        && approximately(
            coverage.analyzed_seconds,
            coverage.analyzed_end_seconds - coverage.analyzed_start_seconds,
        )
        && approximately(
            coverage.ratio,
            coverage.analyzed_seconds / coverage.input_duration_seconds,
        );
    numeric_valid
        && match coverage.status {
            AudioCoverageStatus::Full => {
                coverage.window_count > 0
                    && approximately(coverage.ratio, 1.0)
                    && approximately(coverage.analyzed_start_seconds, 0.0)
                    && approximately(
                        coverage.analyzed_end_seconds,
                        coverage.input_duration_seconds,
                    )
            }
            AudioCoverageStatus::Partial => {
                coverage.window_count > 0
                    && coverage.ratio > 0.0
                    && coverage.ratio < 1.0
                    && coverage.analyzed_seconds > 0.0
            }
            AudioCoverageStatus::None => {
                coverage.window_count == 0
                    && coverage.ratio == 0.0
                    && coverage.analyzed_seconds == 0.0
                    && response.events.is_empty()
                    && response.speech_presence.status == SpeechPresenceStatus::Unknown
            }
        }
}

fn valid_speech_state(response: &WireAudioEventDetection) -> bool {
    let coverage = &response.coverage;
    let policy = &response.policy;
    unit_interval(response.speech_presence.max_score)
        && match response.speech_presence.status {
            SpeechPresenceStatus::Present => {
                response.speech_presence.max_score >= policy.speech_present_threshold
            }
            SpeechPresenceStatus::Absent => {
                coverage.status == AudioCoverageStatus::Full
                    && response.speech_presence.max_score <= policy.speech_absent_threshold
            }
            SpeechPresenceStatus::Unknown => {
                response.speech_presence.max_score < policy.speech_present_threshold
                    && (coverage.status != AudioCoverageStatus::Full
                        || response.speech_presence.max_score > policy.speech_absent_threshold)
            }
        }
}

fn valid_events(response: &WireAudioEventDetection) -> bool {
    let coverage = &response.coverage;
    let policy = &response.policy;
    let mut previous_start = 0.0;
    response.events.iter().enumerate().all(|(index, event)| {
        let valid = !event.class_id.trim().is_empty()
            && !event.label.trim().is_empty()
            && finite_non_negative(event.start_seconds)
            && finite_non_negative(event.end_seconds)
            && event.end_seconds > event.start_seconds
            && event.end_seconds <= coverage.analyzed_end_seconds + 1e-6
            && unit_interval(event.score)
            && event.score >= policy.event_score_threshold
            && (index == 0 || event.start_seconds >= previous_start);
        previous_start = event.start_seconds;
        valid
    })
}

fn validate_local_only_job(job: &RuntimeJobSnapshot) -> Result<(), InferRuntimeError> {
    let constraints = &job.constraints;
    let valid = job.policy == "local-first"
        && job.priority == "background"
        && job.placement == "local"
        && constraints.policy.as_deref() == Some("local-first")
        && constraints.priority.as_deref() == Some("background")
        && constraints.placement.as_deref() == Some("local_only")
        && constraints.prefer.as_deref() == Some("local")
        && constraints.offline_required == Some(true)
        && constraints.quality_floor.as_deref() == Some("basic")
        && constraints.latency.as_deref() == Some("throughput")
        && constraints.max_cost_usd == Some(0.0)
        && constraints.fallback.as_deref() == Some("none")
        && job
            .attempts
            .iter()
            .all(|attempt| attempt.trigger != "fallback");
    if valid {
        return Ok(());
    }
    Err(InferRuntimeError::new(
        InferRuntimeErrorKind::Protocol,
        "inconsistent_audio_event_constraints",
        Some(200),
    ))
}

fn unit_interval(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn finite_non_negative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn approximately(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-6
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests;
