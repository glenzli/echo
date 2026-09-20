//! Progressive analysis levels and evidence contracts.
//!
//! Echo never runs every model at import time. Analysis enriches an asset
//! through stable levels; a record is evidence from a specific model version,
//! not a fact about the recording. Re-analysis is always allowed.

use serde::{Deserialize, Serialize};

/// Progressive analysis stages for one asset. Higher levels depend on lower
/// ones but may arrive independently and may be re-run after a model upgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum AnalysisLevel {
    /// File metadata, duration, waveform peaks.
    Metadata = 0,
    /// Voice-activity detection and basic audio classification.
    Vad = 1,
    /// Speech-to-text transcription.
    Asr = 2,
    /// Speaker, emotion, and audio-event understanding.
    Understanding = 3,
    /// CLAP-style embedding and semantic indexing.
    SemanticIndex = 4,
    /// LLM contextual understanding and memory association.
    Contextual = 5,
}

/// All levels in ascending order, for job planning.
pub const ALL_ANALYSIS_LEVELS: [AnalysisLevel; 6] = [
    AnalysisLevel::Metadata,
    AnalysisLevel::Vad,
    AnalysisLevel::Asr,
    AnalysisLevel::Understanding,
    AnalysisLevel::SemanticIndex,
    AnalysisLevel::Contextual,
];

impl TryFrom<u8> for AnalysisLevel {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Metadata),
            1 => Ok(Self::Vad),
            2 => Ok(Self::Asr),
            3 => Ok(Self::Understanding),
            4 => Ok(Self::SemanticIndex),
            5 => Ok(Self::Contextual),
            other => Err(other),
        }
    }
}

/// What a record describes about an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalysisKind {
    /// Word/sentence-level transcript segments with timestamps.
    Transcript,
    /// Explicit original-range evidence; does not satisfy whole-source ASR.
    SelectionTranscript,
    /// Forced-alignment timing evidence for transcript words or units.
    Alignment,
    /// Speaker turns or diarization segments.
    Speakers,
    /// Emotion observations.
    Emotions,
    /// Audio events (laughter, applause, rain, trains, ...).
    AudioEvents,
    /// Text or audio embedding payload reference.
    Embedding,
    /// Semantic segment index.
    Semantic,
    /// LLM contextual understanding: summary, keywords, mood, place hints.
    Contextual,
}

/// A specific model release that produced evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelIdentity {
    pub name: String,
    pub version: String,
}

impl ModelIdentity {
    /// Creates a model identity.
    #[must_use]
    pub const fn new(name: String, version: String) -> Self {
        Self { name, version }
    }
}

/// One piece of analysis evidence: value from a known model at a known time.
///
/// `value` is a versioned JSON payload whose schema is owned by the analysis
/// kind's semantic owner; readers must tolerate older model versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisRecord {
    pub kind: AnalysisKind,
    pub value: serde_json::Value,
    pub model: ModelIdentity,
    /// Model-reported confidence in `[0, 1]`, or `None` when the model does
    /// not expose one.
    pub confidence: Option<f64>,
    /// Unix milliseconds when the analysis ran.
    pub recorded_at_millis: i64,
}

impl AnalysisRecord {
    /// Creates an evidence record.
    #[must_use]
    pub fn new(
        kind: AnalysisKind,
        value: serde_json::Value,
        model: ModelIdentity,
        confidence: Option<f64>,
        recorded_at_millis: i64,
    ) -> Self {
        Self {
            kind,
            value,
            model,
            confidence,
            recorded_at_millis,
        }
    }
}
