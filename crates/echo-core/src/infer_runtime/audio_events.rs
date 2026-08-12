//! Echo sound-event evidence types.
//!
//! The frozen official Capability Catalog does not publish an
//! `audio.detect_events` capability. Echo therefore preserves its product and
//! persistence types but fails closed instead of retaining a private wire path.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    InferRuntimeClient, InferRuntimeError, InferRuntimeErrorKind, RuntimeProvenance,
    default_background_constraints,
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
    pub speech_present_threshold: f64,
    pub speech_absent_threshold: f64,
}

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
    /// Fails closed until the official Catalog and SDK publish this capability.
    ///
    /// # Errors
    ///
    /// Always returns `capability_contract_unsupported` without reading input.
    pub fn detect_audio_events(
        &self,
        _source: &std::path::Path,
        _intent: &AudioEventDetectionIntent,
    ) -> Result<AudioEventDetection, InferRuntimeError> {
        Err(InferRuntimeError::new(
            InferRuntimeErrorKind::ContractMismatch,
            "capability_contract_unsupported",
            None,
        ))
    }
}

#[cfg(test)]
mod tests;
