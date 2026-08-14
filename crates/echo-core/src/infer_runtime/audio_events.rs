//! Echo sound-event evidence types.
//!
//! This owner maps the dated official SDK response into Echo's durable evidence
//! shape. The SDK remains the sole owner of Discovery, HTTP and strict
//! capability response validation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use infer_runtime_client::{
    AudioCoverageStatus as SdkAudioCoverageStatus,
    AudioEventDetectionResponse as SdkAudioEventDetectionResponse,
    SpeechPresenceStatus as SdkSpeechPresenceStatus,
};

use super::{
    InferRuntimeClient, InferRuntimeError, RuntimeProvenance, default_background_constraints,
};

pub const AUDIO_EVENT_DETECTION_INTENT: &str = "audio.detect_events";

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectedAudioEvent {
    pub class_id: String,
    pub label: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeechPresenceStatus {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechPresence {
    pub status: SpeechPresenceStatus,
    pub max_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCoverageStatus {
    Full,
    Partial,
    None,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventSmoothingPolicy {
    pub method: String,
    pub window_frames: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventDetectionPolicy {
    pub revision: String,
    pub score_kind: String,
    pub event_score_threshold: f64,
    pub smoothing: SoundEventSmoothingPolicy,
    pub max_classes_per_window: usize,
    pub max_events: usize,
    pub speech_class_set_revision: String,
    pub speech_present_threshold: f64,
    pub speech_absent_threshold: f64,
    pub max_audio_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundEventProvenance {
    pub model: String,
    pub model_archive_sha256: String,
    pub artifact_set_sha256: String,
    pub model_license_spdx: String,
    pub training_data_license_spdx: String,
    pub runtime: String,
    pub runtime_version: String,
    pub decoder: String,
    pub decoder_version: String,
    pub preprocessing_identity: String,
}

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

impl InferRuntimeClient {
    /// Submits the dated official sound-event detection intent.
    ///
    /// # Errors
    ///
    pub fn detect_audio_events(
        &self,
        source: &std::path::Path,
        intent: &AudioEventDetectionIntent,
    ) -> Result<AudioEventDetection, InferRuntimeError> {
        super::validate_source(source)?;
        if intent.model != AUDIO_EVENT_DETECTION_INTENT {
            return Err(super::rejected("invalid_audio_event_detection_intent"));
        }
        let (response, job) = self
            .transport()?
            .detect_audio_events(source, &intent.metadata)?;
        let job = super::validate_succeeded_job(
            job,
            AUDIO_EVENT_DETECTION_INTENT,
            super::AUDIO_EVENT_DETECTION_CAPABILITY,
        )?;
        super::validate_local_only_job(&job, "inconsistent_audio_event_constraints")?;
        Ok(AudioEventDetection::from_sdk(
            response,
            RuntimeProvenance {
                contract_version: super::EXPECTED_CONTRACT_VERSION.to_owned(),
                job,
            },
        ))
    }
}

impl AudioEventDetection {
    fn from_sdk(response: SdkAudioEventDetectionResponse, runtime: RuntimeProvenance) -> Self {
        Self {
            id: response.id,
            model: response.model,
            object: response.object,
            events: response
                .events
                .into_iter()
                .map(|event| DetectedAudioEvent {
                    class_id: event.class_id,
                    label: event.label,
                    start_seconds: event.start_seconds,
                    end_seconds: event.end_seconds,
                    score: event.score,
                })
                .collect(),
            speech_presence: SpeechPresence {
                status: match response.speech_presence.status {
                    SdkSpeechPresenceStatus::Present => SpeechPresenceStatus::Present,
                    SdkSpeechPresenceStatus::Absent => SpeechPresenceStatus::Absent,
                    SdkSpeechPresenceStatus::Unknown => SpeechPresenceStatus::Unknown,
                },
                max_score: response.speech_presence.max_score,
            },
            coverage: AudioAnalysisCoverage {
                status: match response.coverage.status {
                    SdkAudioCoverageStatus::Full => AudioCoverageStatus::Full,
                    SdkAudioCoverageStatus::Partial => AudioCoverageStatus::Partial,
                    SdkAudioCoverageStatus::None => AudioCoverageStatus::None,
                },
                input_duration_seconds: response.coverage.input_duration_seconds,
                analyzed_start_seconds: response.coverage.analyzed_start_seconds,
                analyzed_end_seconds: response.coverage.analyzed_end_seconds,
                analyzed_seconds: response.coverage.analyzed_seconds,
                ratio: response.coverage.ratio,
                window_count: response.coverage.window_count,
                window_seconds: response.coverage.window_seconds,
                hop_seconds: response.coverage.hop_seconds,
            },
            ontology: SoundEventOntology {
                id: response.ontology.id,
                revision: response.ontology.revision,
                class_id_namespace: response.ontology.class_id_namespace,
                class_count: response.ontology.class_count,
                artifact_sha256: response.ontology.artifact_sha256,
                license_spdx: response.ontology.license_spdx,
            },
            policy: SoundEventDetectionPolicy {
                revision: response.policy.revision,
                score_kind: response.policy.score_kind,
                event_score_threshold: response.policy.event_score_threshold,
                smoothing: SoundEventSmoothingPolicy {
                    method: response.policy.smoothing.method,
                    window_frames: response.policy.smoothing.window_frames,
                },
                max_classes_per_window: response.policy.max_classes_per_window,
                max_events: response.policy.max_events,
                speech_class_set_revision: response.policy.speech_class_set_revision,
                speech_present_threshold: response.policy.speech_present_threshold,
                speech_absent_threshold: response.policy.speech_absent_threshold,
                max_audio_seconds: response.policy.max_audio_seconds,
            },
            provenance: SoundEventProvenance {
                model: response.provenance.model,
                model_archive_sha256: response.provenance.model_archive_sha256,
                artifact_set_sha256: response.provenance.artifact_set_sha256,
                model_license_spdx: response.provenance.model_license_spdx,
                training_data_license_spdx: response.provenance.training_data_license_spdx,
                runtime: response.provenance.runtime,
                runtime_version: response.provenance.runtime_version,
                decoder: response.provenance.decoder,
                decoder_version: response.provenance.decoder_version,
                preprocessing_identity: response.provenance.preprocessing_identity,
            },
            runtime,
        }
    }
}

#[cfg(test)]
mod tests;
