//! Core workflows: importing real recordings into the Library.
//!
//! This crate orchestrates the catalog, the cache, and (later) the C++ audio
//! engine and AI workers. It knows nothing about Qt.
//!
//! Start with [`import`] for the idempotent import path. Background job
//! scheduling arrives with the first real analysis consumer (see ROADMAP
//! M0); no scheduler exists before there are jobs to run.

mod analysis;
mod contextual;
mod error;
mod import;
mod scanner;
mod util;
mod waveform_artifact;
mod worker;

pub use analysis::{
    TranscribeWorker, TranscriptPayload, TranscriptSegment, TranscriptWord, record_transcript,
    run_transcribe,
};
pub use contextual::{ContextualPayload, ContextualWorker, record_contextual, run_contextual};
pub use error::CoreErrorKind;
pub use import::{ImportOutcome, import_asset, import_asset_with_probe};
pub use scanner::{
    FolderScanner, ScanOutcome, add_root_and_scan, queue_scans_for_enabled_roots, scan_root,
};
pub use waveform_artifact::{
    WaveformArtifact, WaveformArtifactLevel, WaveformArtifactPayload, build_and_cache_waveform,
};
pub use worker::{WorkerConfig, WorkerPool};

#[cfg(test)]
mod tests;
