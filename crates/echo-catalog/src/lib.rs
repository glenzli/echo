//! SQLite-backed catalog persistence.
//!
//! This crate owns the current schema and write transactions. It deliberately
//! knows nothing about Qt, `FFmpeg`, playback, or analysis models.
//!
//! Start with [`catalog`] for connection lifecycle and schema ownership,
//! [`asset_registration`] for idempotent source registration, [`analysis`]
//! for append-only analysis evidence, [`job_queue`] for the persistent
//! background work queue, [`adjustment_graph`] for authored non-destructive
//! revisions, [`processing_recipe`] for reusable processing definitions and
//! explicit batch receipts, [`user_albums`] for durable user-authored
//! collections, [`sound_library`] for memory/material membership and accepted mixes, and [`scan_root`] for the directories Echo watches.

mod adjustment_graph;
mod analysis;
mod analysis_recovery;
mod asset_affinity;
mod asset_path;
mod asset_registration;
mod audio_semantic_index;
mod audio_space;
mod catalog;
mod contextual_facets;
mod derived_artifact;
mod error;
mod generated_audio;
mod impulse_response;
mod inference_run;
mod job_queue;
mod listening_state;
mod long_audio;
mod metadata_calibration;
mod processing_recipe;
mod render_export;
mod rendered_spectral_working_copy;
mod revisit;
mod scan_journal;
mod scan_root;
mod schema;
mod search;
mod semantic_search;
mod smart_albums;
mod sound_assembly;
mod sound_library;
mod source_disclosure;
mod source_metadata;
mod user_albums;

pub use adjustment_graph::{
    AssetAdjustmentRevision, adjustment_graph_at_revision, latest_adjustment_graph,
    record_adjustment_graph, record_project_adjustment_graph,
};
pub use analysis::{
    AnalysisQueryError, AppendAnalysisRecord, list_assets_missing_analysis,
    list_assets_with_alignment_missing_current_contextual,
    list_assets_with_empty_latest_transcript,
    list_assets_with_empty_transcript_missing_audio_events,
    list_assets_with_nonempty_transcript_missing_alignment, query_analysis, record_analysis,
};
pub use analysis_recovery::{
    AnalysisRecoveryMode, AnalysisStage, AssetAnalysisStatus, analysis_recovery_mode,
    automatic_analysis_recovery_count, list_asset_analysis_statuses, requeue_automatic_analysis,
    requeue_manual_analysis,
};
pub use asset_affinity::{AssetAffinity, asset_affinity, set_asset_affinity};
pub use asset_path::{mark_asset_missing, mark_asset_present, relink_asset_by_hash};
pub use asset_registration::{
    AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash, find_by_id,
    list_assets, register_asset,
};
pub use audio_semantic_index::{
    AudioSemanticSearchHit, AudioSemanticSource, list_audio_sources_needing_embedding,
    search_audio_semantic_segments, upsert_audio_semantic_segment,
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
pub use impulse_response::{
    ImpulseResponseLayout, ImpulseResponseRecord, ImpulseResponseRights, list_impulse_responses,
    record_impulse_response,
};
pub use inference_run::{
    InferenceRun, InferenceRunState, UpsertInferenceRun, inference_run, upsert_inference_run,
};
pub use job_queue::{
    ClaimedJob, FileJobPayload, Job, JobKind, JobState, JobStats, MaterialImportPayload,
    ScanRootJobPayload, claim_next_job, complete_job, enqueue_job, fail_job, job_by_id, job_stats,
    list_failed_jobs, recover_interrupted_jobs, requeue_scan_job, retry_job, update_job_progress,
};
pub use listening_state::{
    AssetListeningState, asset_listening_state, record_asset_listening_progress,
};
pub use long_audio::{
    LongAudioOutlineNode, LongAudioProxyRef, LongAudioSegment, LongAudioSegmentPlan,
    LongAudioStage, ensure_long_audio_plan, list_long_audio_outline_nodes,
    list_long_audio_segments, record_long_audio_proxy, record_long_audio_stage,
    upsert_long_audio_outline_node,
};
pub use metadata_calibration::{
    MetadataCalibrationRevision, calibrate_asset_metadata, latest_metadata_calibration,
};
pub use processing_recipe::{
    CreateProcessingRecipe, ProcessingRecipe, ProcessingRecipeApplicationHistoryEntry,
    ProcessingRecipeApplicationReceipt, ProcessingRecipeApplicationRevertReceipt,
    ProcessingRecipeApplicationRevertSummary, ProcessingRecipeRevertTargetOutcome,
    ProcessingRecipeRevertTargetReceipt, ProcessingRecipeTargetOutcome,
    ProcessingRecipeTargetReceipt, append_processing_recipe_revision, apply_processing_recipe,
    archive_processing_recipe, create_processing_recipe,
    list_processing_recipe_application_history, list_processing_recipes, processing_recipe,
    processing_recipe_application_receipt, processing_recipe_application_revert_receipt,
    processing_recipe_patch_from_asset, rename_processing_recipe,
    revert_processing_recipe_application,
};
pub use render_export::{
    RecordRenderExport, RenderExportFormat, RenderExportRecord, list_render_exports,
    record_render_export, record_rendered_spectral_working_copy_export,
};
pub use rendered_spectral_working_copy::{
    CommitRenderedSpectralErase, CreateRenderedSpectralWorkingCopy, RenderedSpectralWorkingCopy,
    RenderedSpectralWorkingCopyAvailability, commit_rendered_spectral_erase,
    create_rendered_spectral_working_copy, list_rendered_spectral_working_copies,
    remove_rendered_spectral_working_copy, set_rendered_spectral_working_copy_enabled,
};
pub use revisit::{RevisitSnapshot, revisit_snapshot};
pub use scan_journal::{journal_fingerprint, upsert_journal};
pub use scan_root::{ScanRoot, add_scan_root, list_scan_roots, remove_scan_root};
pub use schema::{CatalogSchemaRevision, CatalogSchemaRevisionParseError};
pub use search::{
    SearchHit, index_transcript, remove_transcript_index, search_semantic_text, search_transcripts,
    segment_cjk,
};
pub use semantic_search::{
    SemanticSearchHit, SemanticSource, UpsertSemanticDocument, index_semantic_source_text,
    list_semantic_sources_needing_embedding, remove_semantic_documents_outside_contract,
    search_semantic_documents, semantic_source, upsert_semantic_document,
};
pub use smart_albums::{
    SmartAlbumCandidate, SmartAlbumEvidence, SmartAlbumFacet, list_smart_album_candidates,
};
pub use sound_assembly::{
    RecordSoundAssemblyExport, SoundAssemblyExportFormat, SoundAssemblyExportRecord,
    SoundAssemblyRevision, SoundAssemblySummary, archive_sound_assembly, latest_sound_assembly,
    list_sound_assemblies, record_sound_assembly, record_sound_assembly_export,
    sound_assembly_at_revision,
};
pub use source_metadata::{
    SourceMetadata, SourceMetadataEntry, list_assets_missing_source_metadata,
    record_source_metadata,
};
pub use user_albums::{
    CreateUserAlbum, UserAlbum, UserAlbumId, create_user_album, delete_user_album,
    list_user_albums, rename_user_album, set_user_album_membership,
};

pub use sound_library::{
    AssemblyMemory, SoundMembership, assembly_memories, assembly_memory_path,
    attach_project_material, preserve_assembly_memory, project_material_ids, set_sound_membership,
    sound_memberships,
};

pub use generated_audio::record_generated_audio;
pub use source_disclosure::{
    SourceDisclosureOrigin, SourceDisclosureRevision, SourceDisclosureSummary,
    assembly_source_disclosure, asset_source_disclosure, record_source_disclosure,
    source_disclosures,
};

#[cfg(test)]
mod tests;
