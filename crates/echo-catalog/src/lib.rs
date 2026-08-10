//! SQLite-backed catalog persistence.
//!
//! This crate owns the current schema and write transactions. It deliberately
//! knows nothing about Qt, `FFmpeg`, playback, or analysis models.
//!
//! Start with [`catalog`] for connection lifecycle and schema ownership,
//! [`asset_registration`] for idempotent source registration, [`analysis`]
//! for append-only analysis evidence, [`job_queue`] for the persistent
//! background work queue, [`adjustment_graph`] for authored non-destructive
//! revisions, [`user_albums`] for durable user-authored collections, and
//! [`scan_root`] for the directories Echo watches.

mod adjustment_graph;
mod analysis;
mod asset_affinity;
mod asset_path;
mod asset_registration;
mod audio_space;
mod catalog;
mod contextual_facets;
mod derived_artifact;
mod error;
mod inference_run;
mod job_queue;
mod long_audio;
mod render_export;
mod scan_journal;
mod scan_root;
mod schema;
mod search;
mod semantic_search;
mod smart_albums;
mod source_metadata;
mod user_albums;

pub use adjustment_graph::{
    AssetAdjustmentRevision, latest_adjustment_graph, record_adjustment_graph,
};
pub use analysis::{
    AnalysisQueryError, AppendAnalysisRecord, list_assets_missing_analysis,
    list_assets_with_alignment_missing_current_contextual,
    list_assets_with_empty_latest_transcript,
    list_assets_with_empty_transcript_missing_audio_events,
    list_assets_with_nonempty_transcript_missing_alignment, query_analysis, record_analysis,
};
pub use asset_affinity::{AssetAffinity, asset_affinity, set_asset_affinity};
pub use asset_path::{mark_asset_missing, mark_asset_present, relink_asset_by_hash};
pub use asset_registration::{
    AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash, find_by_id,
    list_assets, register_asset,
};
pub use audio_space::{AudioSpaceAsset, list_audio_space};
pub use catalog::{Catalog, CatalogStats, open_catalog};
pub use contextual_facets::{
    AppendContextualAnalysis, ContextualKeywordFacet, list_contextual_keyword_facets,
    record_contextual_analysis,
};
pub use derived_artifact::{
    DerivedArtifactKind, DerivedArtifactRecord, find_derived_artifact, remove_derived_artifact,
    upsert_derived_artifact,
};
pub use error::CatalogError;
pub use error::CatalogErrorKind;
pub use inference_run::{
    InferenceRun, InferenceRunState, UpsertInferenceRun, inference_run,
    requeue_recoverable_inference_runs, upsert_inference_run,
};
pub use job_queue::{
    ClaimedJob, FileJobPayload, Job, JobKind, JobState, JobStats, ScanRootJobPayload,
    claim_next_job, complete_job, enqueue_job, fail_job, job_by_id, job_stats, list_failed_jobs,
    recover_interrupted_jobs, requeue_scan_job, retry_job, update_job_progress,
};
pub use long_audio::{
    LongAudioOutlineNode, LongAudioProxyRef, LongAudioSegment, LongAudioSegmentPlan,
    LongAudioStage, ensure_long_audio_plan, list_long_audio_outline_nodes,
    list_long_audio_segments, record_long_audio_proxy, record_long_audio_stage,
    upsert_long_audio_outline_node,
};
pub use render_export::{
    RecordRenderExport, RenderExportFormat, RenderExportRecord, list_render_exports,
    record_render_export,
};
pub use scan_journal::{journal_fingerprint, upsert_journal};
pub use scan_root::{ScanRoot, add_scan_root, list_scan_roots, remove_scan_root};
pub use schema::{CatalogSchemaRevision, CatalogSchemaRevisionParseError};
pub use search::{
    SearchHit, index_transcript, remove_transcript_index, search_semantic_text, search_transcripts,
    segment_cjk,
};
pub use semantic_search::{
    SemanticSearchHit, SemanticSource, UpsertSemanticDocument, index_semantic_source_text,
    list_semantic_sources_needing_embedding, search_semantic_documents, semantic_source,
    upsert_semantic_document,
};
pub use smart_albums::{
    SmartAlbumCandidate, SmartAlbumEvidence, SmartAlbumFacet, list_smart_album_candidates,
};
pub use source_metadata::{
    SourceMetadata, SourceMetadataEntry, list_assets_missing_source_metadata,
    record_source_metadata,
};
pub use user_albums::{
    CreateUserAlbum, UserAlbum, UserAlbumId, create_user_album, delete_user_album,
    list_user_albums, rename_user_album, set_user_album_membership,
};

#[cfg(test)]
mod tests;
