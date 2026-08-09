//! Core workflows: importing real recordings into the Library.
//!
//! This crate orchestrates the catalog, the cache, and (later) the C++ audio
//! engine and AI workers. It knows nothing about Qt.
//!
//! Start with [`import`] for the idempotent import path. The background queue
//! performs rebuildable Level 0 work; model execution remains behind an
//! inference boundary and never enters playback.

mod analysis;
mod contextual;
mod error;
mod import;
mod scanner;
mod util;
mod waveform_artifact;
mod worker;

pub use analysis::{
    TRANSCRIPTION_INTENT, TranscribeWorker, TranscriptPayload, TranscriptSegment, TranscriptWord,
    TranscriptionIntent, record_transcript, run_transcribe,
};
pub use contextual::{ContextualPayload, ContextualWorker, record_contextual, run_contextual};
pub use error::CoreErrorKind;
pub use import::{ImportOutcome, import_asset, import_asset_with_probe};
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
