//! SQLite-backed catalog persistence.
//!
//! This crate owns the current schema and write transactions. It deliberately
//! knows nothing about Qt, `FFmpeg`, playback, or analysis models.
//!
//! Start with [`catalog`] for connection lifecycle and schema ownership,
//! [`asset_registration`] for idempotent source registration, [`analysis`]
//! for append-only analysis evidence, [`job_queue`] for the persistent
//! background work queue, and [`scan_root`] for the directories Echo watches.

mod analysis;
mod asset_path;
mod asset_registration;
mod catalog;
mod error;
mod job_queue;
mod scan_journal;
mod scan_root;
mod schema;

pub use analysis::{AnalysisQueryError, AppendAnalysisRecord, query_analysis, record_analysis};
pub use asset_path::{AssetPathStatus, mark_asset_missing, relink_asset_by_hash};
pub use asset_registration::{
    AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash, find_by_id,
    list_assets, register_asset,
};
pub use catalog::{Catalog, CatalogStats, open_catalog};
pub use error::CatalogError;
pub use error::CatalogErrorKind;
pub use job_queue::{
    ClaimedJob, FileJobPayload, Job, JobKind, JobState, JobStats, ScanRootJobPayload,
    claim_next_job, complete_job, enqueue_job, fail_job, job_stats, list_failed_jobs,
    recover_interrupted_jobs, update_job_progress,
};
pub use scan_journal::{journal_fingerprint, upsert_journal};
pub use scan_root::{ScanRoot, add_scan_root, list_scan_roots, remove_scan_root};

#[cfg(test)]
mod tests;
