//! Core workflows: importing real recordings into the Library.
//!
//! This crate orchestrates the catalog, the cache, and (later) the C++ audio
//! engine and AI workers. It knows nothing about Qt.
//!
//! [`generated_narration`] owns transient speech candidates and explicit durable admission.
//! [`material_import`] owns durable global and project material intake.
//! [`editor_transcription`] owns explicit original-range AI evidence without
//! starting automatic Library jobs. Start with [`import`] for the idempotent import path. The background queue
//! performs rebuildable Level 0 work; model execution remains behind an
//! inference boundary and never enters playback.

mod analysis;
mod analysis_queue;
mod analysis_recovery;
mod audio_semantic_search;
mod contextual;
mod editor_transcription;
mod error;
mod generated_narration;
mod import;
mod infer_runtime;
mod infer_runtime_credentials;
mod long_audio;
mod material_import;
mod metadata_queue;
mod scanner;
mod search;
mod semantic_search;
mod sound_event_workflow;
mod sound_events;
mod spectrogram_artifact;
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
pub use editor_transcription::{
    MAX_SELECTION_TRANSCRIPTION_MILLIS, SelectionTranscript, record_selection_transcript,
    transcribe_selection,
};
pub use error::{CoreError, CoreErrorKind};
pub use generated_narration::{NarrationCandidate, accept_narration, generate_narration};
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
pub use spectrogram_artifact::{
    SpectrogramArtifact, SpectrogramArtifactPayload, build_and_cache_spectrogram,
    load_or_build_spectrogram,
};
pub use waveform_artifact::{
    WaveformArtifact, WaveformArtifactLevel, WaveformArtifactPayload, build_and_cache_waveform,
    load_or_build_memory_waveform, load_or_build_waveform,
};
pub use worker::{WorkerConfig, WorkerPool};

#[cfg(test)]
mod tests;
