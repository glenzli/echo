//! Core workflows: importing real recordings into the Library.
//!
//! This crate orchestrates the catalog, the cache, and (later) the C++ audio
//! engine and AI workers. It knows nothing about Qt.
//!
//! Start with [`import`] for the idempotent import path. The background queue
//! performs rebuildable Level 0 work; model execution remains behind an
//! inference boundary and never enters playback.

mod analysis;
mod analysis_queue;
mod analysis_recovery;
mod audio_semantic_search;
mod contextual;
mod error;
mod import;
mod infer_runtime;
mod infer_runtime_credentials;
mod long_audio;
mod metadata_queue;
mod scanner;
mod search;
mod semantic_search;
mod sound_event_workflow;
mod sound_events;
mod util;
mod waveform_artifact;
mod worker;

pub use analysis::{
    TranscriptPayload, TranscriptSegment, TranscriptWord, record_alignment,
    record_runtime_transcript, record_transcript,
};
pub use analysis_queue::contextual_job_id;
pub use contextual::{
    CONTEXTUAL_JOB_REVISION, CONTEXTUAL_SCHEMA_VERSION, ContextualPayload, record_contextual,
};
pub use error::{CoreError, CoreErrorKind};
pub use import::{ImportOutcome, hash_file, import_asset, import_asset_with_probe};
pub use infer_runtime::{
    ALIGNMENT_INTENT, AUDIO_EMBEDDING_INTENT, AUDIO_EVENT_DETECTION_INTENT,
    AUDIO_TEXT_QUERY_EMBEDDING_INTENT, AlignmentIntent, AlignmentItem, AlignmentPayload,
    AudioAnalysisCoverage, AudioCoverageStatus, AudioEmbeddingPayload, AudioEventDetection,
    AudioEventDetectionIntent, AudioTextQueryEmbeddingIntent, CONTEXTUAL_INTENT, ContextualIntent,
    ContextualResponse, DetectedAudioEvent, EXPECTED_CONTRACT_VERSION, InferRuntimeClient,
    InferRuntimeConfig, InferRuntimeError, InferRuntimeErrorKind, MAX_AUDIO_UPLOAD_BYTES,
    MAX_CONTEXTUAL_INPUT_BYTES, RuntimeAttempt, RuntimeCandidateDecision, RuntimeJobConstraints,
    RuntimeJobSnapshot, RuntimeProvenance, RuntimeRoutingDecision, SoundEventDetectionPolicy,
    SoundEventOntology, SoundEventProvenance, SoundEventSmoothingPolicy, SpeechPresence,
    SpeechPresenceStatus, TEXT_EMBEDDING_INTENT, TRANSCRIPTION_INTENT, TextEmbeddingIntent,
    TextEmbeddingPayload, TextEmbeddingProviderProvenance, TranscriptionIntent,
};
pub use infer_runtime_credentials::{
    InferRuntimeCredentialError, InferRuntimeCredentialStore, infer_runtime_credential_available,
    infer_runtime_credential_path,
};
pub use long_audio::LONG_AUDIO_PLAN_VERSION;
pub use scanner::{
    FolderScanner, ScanOutcome, add_root_and_scan, queue_scans_for_enabled_roots, scan_root,
};
pub use search::search as semantic_search;
pub use sound_events::{
    AUDIO_EVENTS_SCHEMA_VERSION, AudioEventChunk, AudioEventsEvidence, record_audio_events,
};
pub use waveform_artifact::{
    WaveformArtifact, WaveformArtifactLevel, WaveformArtifactPayload, build_and_cache_waveform,
    load_or_build_waveform,
};
pub use worker::{WorkerConfig, WorkerPool};

#[cfg(test)]
mod tests;
