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
mod contextual;
mod error;
mod import;
mod infer_runtime;
mod infer_runtime_credentials;
mod metadata_queue;
mod scanner;
mod util;
mod waveform_artifact;
mod worker;

pub use analysis::{
    TranscriptPayload, TranscriptSegment, TranscriptWord, record_alignment,
    record_runtime_transcript, record_transcript,
};
pub use contextual::{ContextualPayload, record_contextual};
pub use error::CoreErrorKind;
pub use import::{ImportOutcome, import_asset, import_asset_with_probe};
pub use infer_runtime::{
    ALIGNMENT_INTENT, AlignmentIntent, AlignmentItem, AlignmentPayload, CONTEXTUAL_INTENT,
    ContextualIntent, ContextualResponse, EXPECTED_CONTRACT_VERSION, InferRuntimeClient,
    InferRuntimeConfig, InferRuntimeError, InferRuntimeErrorKind, MAX_AUDIO_UPLOAD_BYTES,
    MAX_CONTEXTUAL_INPUT_BYTES, RuntimeAttempt, RuntimeCandidateDecision, RuntimeJobConstraints,
    RuntimeJobSnapshot, RuntimeProvenance, RuntimeRoutingDecision, TRANSCRIPTION_INTENT,
    TranscriptionIntent,
};
pub use infer_runtime_credentials::{
    InferRuntimeCredential, InferRuntimeCredentialError, InferRuntimeCredentialStore,
    infer_runtime_credential_available, load_infer_runtime_credential,
};
pub use scanner::{
    FolderScanner, ScanOutcome, add_root_and_scan, queue_scans_for_enabled_roots, scan_root,
};
pub use waveform_artifact::{
    WaveformArtifact, WaveformArtifactLevel, WaveformArtifactPayload, build_and_cache_waveform,
    load_or_build_waveform,
};
pub use worker::{WorkerConfig, WorkerPool};

#[cfg(test)]
mod tests;
