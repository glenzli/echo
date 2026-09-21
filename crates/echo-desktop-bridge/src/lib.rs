//! Long-lived desktop services bridging Qt to the Rust memory engine.
//!
//! QML and desktop controllers never open `SQLite`, call `FFmpeg`, or interpret
//! cache paths: they talk to this crate through the generated CXX ABI, and it
//! owns the durable [`LibrarySession`] lifecycle.

mod editor_project;
mod editor_session;
mod generated_narration;
mod render_exports;
mod selection_transcription;
use generated_narration::NarrationCandidate;
mod session;

use crate::session::LibrarySession;
use std::path::Path;

#[cxx::bridge(namespace = "echo::desktop")]
mod ffi {
    /// One authored parametric equalizer band crossing the desktop ABI.
    #[derive(Debug)]
    struct EqualizerBandWire {
        enabled: bool,
        filter_kind: u8,
        frequency_hertz: u16,
        q_hundredths: u16,
        gain_centibels: i16,
    }

    /// One source-anchored segment in the non-destructive edit timeline.
    #[derive(Debug)]
    struct EditSegmentWire {
        source_start_millis: u64,
        source_end_millis: u64,
        state: u8,
        gain_centibels: i16,
        fade_in_millis: u64,
        fade_out_millis: u64,
        fade_in_curve: u8,
        fade_out_curve: u8,
        gap_after_millis: u64,
    }

    /// One original-time range that scopes a bounded set of insert effects.
    #[derive(Debug)]
    struct EffectMaskWire {
        start_millis: u64,
        end_millis: u64,
        feather_millis: u16,
        effect_nodes: Vec<u8>,
    }

    /// User-owned listening continuity returned after one bounded checkpoint.
    #[derive(Debug)]
    struct AssetListeningStateWire {
        last_listened_at_millis: i64,
        resume_position_millis: u64,
    }

    /// Rights-tracked local impulse response ready for the Space node.
    #[derive(Debug)]
    struct ImpulseResponseWire {
        import_id: String,
        source_hash: String,
        prepared_hash: String,
        prepared_path: String,
        display_name: String,
        creator: String,
        source_url: String,
        attribution: String,
        rights_kind: String,
        spdx_expression: String,
        license_url: String,
        imported_at_millis: u64,
        source_sample_rate: u32,
        channel_count: u32,
        layout_kind: String,
        preparation_version: u32,
        prepared_frame_count: u64,
    }

    /// Bounded presentation projection of one asset for the desktop shell.
    #[derive(Debug)]
    struct AssetSummaryWire {
        id: String,
        in_memory: bool,
        in_materials: bool,
        material_category: String,
        assembly_id: String,
        assembly_revision_id: i64,
        provenance_json: String,
        source_disclosure_json: String,
        has_generated_source: bool,
        has_ai_processed_source: bool,
        path: String,
        codec: String,
        duration_millis: u64,
        recorded_at_millis: i64,
        imported_at_millis: i64,
        max_level: u8,
        path_status: String,
        sound_caption: String,
        summary: String,
        event_type: String,
        mood: String,
        keywords: Vec<String>,
        text_preview: String,
        language: String,
        model_sound_caption: String,
        model_summary: String,
        model_event_type: String,
        model_mood: String,
        model_keywords: Vec<String>,
        model_text_preview: String,
        model_language: String,
        calibrated_fields: Vec<String>,
        metadata_calibration_revision: i64,
        analysis_stage: String,
        analysis_state: String,
        analysis_recovery: String,
        analysis_error_code: String,
        analysis_progress: u8,
        analysis_attempts: u32,
        liked: bool,
        rating: u8,
        last_listened_at_millis: i64,
        resume_position_millis: u64,
        adjustment_revision: i64,
        trim_start_millis: u64,
        trim_end_millis: u64,
        fade_in_millis: u64,
        fade_out_millis: u64,
        fade_in_curve: u8,
        fade_out_curve: u8,
        gain_centibels: i16,
        low_cut_hertz: u16,
        restoration_enabled: bool,
        de_plosive_enabled: bool,
        de_plosive_frequency_hertz: u16,
        de_plosive_sensitivity_percent: u8,
        de_plosive_reduction_centibels: u16,
        de_plosive_release_millis: u16,
        noise_reduction_enabled: bool,
        noise_reduction_centibels: u16,
        noise_reduction_sensitivity_percent: u8,
        noise_reduction_smoothing_millis: u16,
        de_esser_enabled: bool,
        de_esser_frequency_hertz: u16,
        de_esser_threshold_centibels: i16,
        de_esser_reduction_centibels: u16,
        de_hum_enabled: bool,
        de_hum_fundamental_hertz: u16,
        de_hum_harmonic_count: u8,
        de_hum_quality_tenths: u16,
        de_hum_depth_centibels: u16,
        de_click_enabled: bool,
        de_click_sensitivity_percent: u8,
        de_click_maximum_click_microseconds: u16,
        de_click_repair_percent: u8,
        channel_repair_enabled: bool,
        channel_repair_invert_left: bool,
        channel_repair_invert_right: bool,
        channel_repair_swap_channels: bool,
        channel_repair_mono_fold_down: bool,
        channel_repair_balance_percent: i8,
        equalizer_enabled: bool,
        equalizer_bands: Vec<EqualizerBandWire>,
        compressor_enabled: bool,
        compressor_threshold_centibels: i16,
        compressor_ratio_tenths: u16,
        compressor_attack_millis: u16,
        compressor_release_millis: u16,
        compressor_makeup_centibels: i16,
        reverb_character: u8,
        reverb_enabled: bool,
        reverb_mix_percent: u8,
        reverb_pre_delay_millis: u16,
        reverb_decay_millis: u16,
        reverb_size_percent: u8,
        reverb_damping_percent: u8,
        reverb_low_cut_hertz: u16,
        reverb_high_cut_hertz: u16,
        reverb_ducking_enabled: bool,
        reverb_ducking_amount_percent: u8,
        reverb_ducking_attack_millis: u16,
        reverb_ducking_release_millis: u16,
        space_mode: u8,
        impulse_response_import_id: String,
        impulse_response_source_hash: String,
        impulse_response_prepared_hash: String,
        impulse_response_prepared_path: String,
        convolution_mix_percent: u8,
        convolution_wet_gain_centibels: i16,
        creative_vfx_json: String,
        spectral_repair_json: String,
        limiter_enabled: bool,
        limiter_ceiling_centibels: i16,
        limiter_release_millis: u16,
        effect_chain: Vec<u8>,
        edit_segments: Vec<EditSegmentWire>,
        effect_masks: Vec<EffectMaskWire>,
        container_format: String,
        sample_rate: u32,
        channel_count: u32,
        source_title: String,
        source_location: String,
        source_created_at: String,
    }

    /// One complete authored adjustment crossing the desktop ABI atomically.
    #[derive(Debug)]
    struct AssetAdjustmentWire {
        trim_start_millis: u64,
        trim_end_millis: u64,
        fade_in_millis: u64,
        fade_out_millis: u64,
        fade_in_curve: u8,
        fade_out_curve: u8,
        gain_centibels: i16,
        low_cut_hertz: u16,
        restoration_enabled: bool,
        de_plosive_enabled: bool,
        de_plosive_frequency_hertz: u16,
        de_plosive_sensitivity_percent: u8,
        de_plosive_reduction_centibels: u16,
        de_plosive_release_millis: u16,
        noise_reduction_enabled: bool,
        noise_reduction_centibels: u16,
        noise_reduction_sensitivity_percent: u8,
        noise_reduction_smoothing_millis: u16,
        de_esser_enabled: bool,
        de_esser_frequency_hertz: u16,
        de_esser_threshold_centibels: i16,
        de_esser_reduction_centibels: u16,
        de_hum_enabled: bool,
        de_hum_fundamental_hertz: u16,
        de_hum_harmonic_count: u8,
        de_hum_quality_tenths: u16,
        de_hum_depth_centibels: u16,
        de_click_enabled: bool,
        de_click_sensitivity_percent: u8,
        de_click_maximum_click_microseconds: u16,
        de_click_repair_percent: u8,
        channel_repair_enabled: bool,
        channel_repair_invert_left: bool,
        channel_repair_invert_right: bool,
        channel_repair_swap_channels: bool,
        channel_repair_mono_fold_down: bool,
        channel_repair_balance_percent: i8,
        equalizer_enabled: bool,
        equalizer_bands: Vec<EqualizerBandWire>,
        compressor_enabled: bool,
        compressor_threshold_centibels: i16,
        compressor_ratio_tenths: u16,
        compressor_attack_millis: u16,
        compressor_release_millis: u16,
        compressor_makeup_centibels: i16,
        reverb_character: u8,
        reverb_enabled: bool,
        reverb_mix_percent: u8,
        reverb_pre_delay_millis: u16,
        reverb_decay_millis: u16,
        reverb_size_percent: u8,
        reverb_damping_percent: u8,
        reverb_low_cut_hertz: u16,
        reverb_high_cut_hertz: u16,
        reverb_ducking_enabled: bool,
        reverb_ducking_amount_percent: u8,
        reverb_ducking_attack_millis: u16,
        reverb_ducking_release_millis: u16,
        space_mode: u8,
        impulse_response_import_id: String,
        impulse_response_source_hash: String,
        impulse_response_prepared_hash: String,
        convolution_mix_percent: u8,
        convolution_wet_gain_centibels: i16,
        creative_vfx_json: String,
        spectral_repair_json: String,
        limiter_enabled: bool,
        limiter_ceiling_centibels: i16,
        limiter_release_millis: u16,
        effect_chain: Vec<u8>,
        edit_segments: Vec<EditSegmentWire>,
        effect_masks: Vec<EffectMaskWire>,
    }

    /// Completed offline-render evidence crossing the desktop ABI atomically.
    #[derive(Debug)]
    struct RenderExportWire {
        source_disclosure_comment: String,
        output_path: String,
        format: String,
        sample_rate: u32,
        channel_count: u32,
        bit_depth: u16,
        frame_count: u64,
        size_bytes: u64,
        integrated_lufs: f32,
        true_peak_dbtp: f32,
    }

    /// Compact top-level Sound Assembly workspace projection.
    #[derive(Debug)]
    struct SoundAssemblySummaryWire {
        assembly_id: String,
        name: String,
        revision_id: i64,
        revision_number: u32,
        duration_millis: u64,
        track_count: u32,
        clip_count: u32,
        updated_at_millis: i64,
    }

    /// One resolved clip source pinned to its exact asset adjustment revision.
    #[derive(Debug)]
    struct SoundAssemblyClipSourceWire {
        clip_id: String,
        path: String,
        adjustment_revision_id: i64,
        impulse_response_prepared_path: String,
        adjustment: AssetAdjustmentWire,
    }

    /// One immutable assembly document plus engine preparation inputs.
    #[derive(Debug)]
    struct SoundAssemblyRevisionWire {
        assembly_id: String,
        revision_id: i64,
        revision_number: u32,
        document_json: String,
        clip_sources: Vec<SoundAssemblyClipSourceWire>,
        created_at_millis: i64,
    }

    /// One desktop projection of a frozen post-effect spectral working copy.
    #[derive(Debug)]
    struct RenderedSpectralWorkingCopyWire {
        id: i64,
        cache_path: String,
        parent_adjustment_revision_id: i64,
        operation_count: u32,
        enabled: bool,
        upstream_current: bool,
    }

    /// One indexed keyword facet over the newest contextual evidence.
    #[derive(Debug)]
    struct KeywordFacetWire {
        key: String,
        label: String,
        count: u64,
    }

    /// One explainable cross-asset album candidate.
    #[derive(Debug)]
    struct SmartAlbumWire {
        key: String,
        label: String,
        facet: String,
        evidence: String,
        count: u64,
        member_asset_ids: Vec<String>,
    }

    /// One user-authored album and its explicit member facts.
    #[derive(Debug)]
    struct UserAlbumWire {
        id: i64,
        name: String,
        cover_asset_id: String,
        count: u64,
        member_asset_ids: Vec<String>,
        created_at_millis: i64,
        updated_at_millis: i64,
    }

    /// Bounded asset identities for the source-anchored Revisit home.
    #[derive(Debug)]
    struct RevisitSnapshotWire {
        continue_listening: Vec<String>,
        recently_listened: Vec<String>,
        on_this_day: Vec<String>,
        recently_added: Vec<String>,
    }

    /// One named reusable processing recipe projected for Qt.
    #[derive(Debug)]
    struct ProcessingRecipeWire {
        id: String,
        name: String,
        revision_id: String,
        revision_number: u32,
        components: Vec<u8>,
        updated_at_millis: i64,
    }

    /// One bounded durable processing-history row for desktop presentation.
    #[derive(Debug)]
    struct ProcessingRecipeHistoryWire {
        batch_id: String,
        recipe_id: String,
        recipe_name: String,
        recipe_revision_id: String,
        recipe_revision_number: u32,
        merge_mode: String,
        target_count: u64,
        updated_count: u64,
        unchanged_count: u64,
        failed_count: u64,
        created_at_millis: i64,
        reverted: bool,
        revert_id: String,
        restored_count: u64,
        revert_unchanged_count: u64,
        conflict_count: u64,
        revert_failed_count: u64,
        reverted_at_millis: i64,
    }

    /// Per-sound outcome from one explicit recipe application batch.
    #[derive(Debug)]
    struct ProcessingRecipeTargetResultWire {
        asset_id: String,
        outcome: String,
        adjustment_revision: i64,
        error: String,
    }

    /// Auditable result of applying one immutable recipe revision.
    #[derive(Debug)]
    struct ProcessingRecipeApplyReceiptWire {
        batch_id: String,
        recipe_revision_id: String,
        updated_count: u64,
        unchanged_count: u64,
        failed_count: u64,
        results: Vec<ProcessingRecipeTargetResultWire>,
    }

    /// Per-sound outcome from safely reverting one application batch.
    #[derive(Debug)]
    struct ProcessingRecipeRevertTargetResultWire {
        asset_id: String,
        outcome: String,
        adjustment_revision: i64,
        error: String,
    }

    /// Durable result of one explicit recipe-application rollback.
    #[derive(Debug)]
    struct ProcessingRecipeRevertReceiptWire {
        revert_id: String,
        application_batch_id: String,
        restored_count: u64,
        unchanged_count: u64,
        conflict_count: u64,
        failed_count: u64,
        results: Vec<ProcessingRecipeRevertTargetResultWire>,
    }

    /// One pyramid level of a cached waveform artifact.
    #[derive(Debug)]
    struct WaveformLevelWire {
        samples_per_bucket: u32,
        mins: Vec<f32>,
        maxs: Vec<f32>,
    }

    /// A cached waveform artifact for display.
    #[derive(Debug)]
    struct WaveformArtifactWire {
        canonical_sample_rate: u32,
        levels: Vec<WaveformLevelWire>,
    }

    /// A cached, bounded spectrogram overview for display.
    #[derive(Debug)]
    struct SpectrogramArtifactWire {
        canonical_sample_rate: u32,
        window_frames: u32,
        hop_frames: u32,
        time_columns: u32,
        frequency_bins: u32,
        magnitudes: Vec<u8>,
    }

    /// One transcript segment for display and click-to-seek.
    #[derive(Debug)]
    struct TranscriptSegmentWire {
        text: String,
        start: f64,
        end: f64,
    }

    /// One transcript evidence record.
    #[derive(Debug)]
    struct TranscriptWire {
        model: String,
        model_version: String,
        language: String,
        text: String,
        segments: Vec<TranscriptSegmentWire>,
    }

    /// One long-recording outline node for source-time navigation.
    #[derive(Debug)]
    struct LongAudioChapterWire {
        level: u32,
        index: u32,
        start_millis: u64,
        end_millis: u64,
        sound_caption: String,
        summary: String,
    }

    /// Aggregate background job statistics.
    #[derive(Debug)]
    struct JobStatsWire {
        pending: u64,
        running: u64,
        done: u64,
        failed: u64,
    }

    /// Per-asset projection of Echo's local stage and Runtime linkage.
    #[derive(Debug)]
    struct AnalysisStatusWire {
        asset_id: String,
        stage: String,
        state: String,
        recovery: String,
        progress: u8,
        attempts: u32,
        error_code: String,
        runtime_job_id: String,
        contract_version: String,
    }

    /// One configured scan root.
    #[derive(Debug)]
    struct ScanRootWire {
        id: i64,
        root: String,
        enabled: bool,
    }

    /// One transcript search hit for the desktop.
    #[derive(Debug)]
    struct SearchHitWire {
        asset_id: String,
        path: String,
        codec: String,
        snippet: String,
        start_millis: u64,
    }

    /// One compact semantic result; presentation resolves the asset itself.
    #[derive(Debug)]
    struct SemanticSearchHitWire {
        asset_id: String,
        score: f64,
    }

    extern "Rust" {
        type LibrarySession;

        /// Reports whether Echo's owner-only Runtime credential is available.
        fn infer_runtime_credential_available() -> bool;
        /// Runs an ephemeral semantic query against one catalog attachment.
        fn semantic_search_catalog(
            catalog_path: &str,
            runtime_endpoint: &str,
            query: &str,
            limit: u64,
        ) -> Result<Vec<SemanticSearchHitWire>>;
        /// Loads or builds one bounded spectrogram overview through a
        /// short-lived catalog attachment. Desktop preview work calls this
        /// from a background worker so a cold derived cache never blocks QML.
        fn spectrogram_artifact_for_catalog(
            catalog_path: &str,
            cache_root: &str,
            asset_id: &str,
        ) -> Result<SpectrogramArtifactWire>;
        /// Builds one bounded display overview for a verified private working
        /// copy. This path is intentionally not catalog-owned: the working
        /// copy cache identity already anchors its mutable lifecycle.
        fn spectrogram_artifact_for_path(path: &str) -> Result<SpectrogramArtifactWire>;
        fn transcribe_editor_selection(
            catalog: &str,
            cache: &str,
            id: &str,
            start: u64,
            end: u64,
            endpoint: &str,
        ) -> Result<String>;
        fn session_accept_selection_transcript(
            self: &LibrarySession,
            id: &str,
            value: &str,
        ) -> Result<()>;
        fn session_selection_transcripts(self: &LibrarySession, id: &str) -> Result<String>;
        type NarrationCandidate;
        fn generate_narration_candidate(
            text: &str,
            directory: &str,
            endpoint: &str,
        ) -> Result<Box<NarrationCandidate>>;
        fn narration_candidate_details(candidate: &NarrationCandidate) -> Result<String>;
        fn accept_narration_candidate(
            catalog: &str,
            candidate: &NarrationCandidate,
            assembly: &str,
            global: bool,
        ) -> Result<String>;
        fn open_editor_session(root: &str) -> Result<Box<LibrarySession>>;
        fn editor_import_audio(root: &str, input: &str) -> Result<String>;
        fn editor_save_project(root: &str, destination: &str) -> Result<()>;
        fn editor_open_project(project: &str, root: &str) -> Result<()>;
        fn session_is_independent(self: &LibrarySession) -> bool;
        /// Opens (creating if needed) the catalog and cache at the given roots.
        fn audio_file_patterns() -> String;
        fn open_session(path: &str, cache_root: &str) -> Result<Box<LibrarySession>>;
        /// Lists registered assets, newest import first.
        fn session_save_project_clip_adjustment(
            self: &LibrarySession,
            document_json: &str,
            clip_id: &str,
            asset_id: &str,
            adjustment: &AssetAdjustmentWire,
        ) -> Result<SoundAssemblyRevisionWire>;
        fn session_list_originals(self: &LibrarySession) -> Result<Vec<AssetSummaryWire>>;
        fn session_list_assets(self: &LibrarySession) -> Result<Vec<AssetSummaryWire>>;
        fn session_export_source_disclosure(
            self: &LibrarySession,
            id: &str,
            assembly_revision: i64,
        ) -> Result<String>;
        fn session_set_source_disclosure(
            self: &LibrarySession,
            id: &str,
            expected_revision: i64,
            spans_json: &str,
        ) -> Result<()>;
        fn session_set_sound_membership(
            self: &LibrarySession,
            id: &str,
            memory: bool,
            materials: bool,
            category: &str,
        ) -> Result<()>;
        fn session_project_materials(
            self: &LibrarySession,
            assembly_id: &str,
        ) -> Result<Vec<String>>;
        fn session_queue_material_import(
            self: &LibrarySession,
            path: &str,
            assembly_id: &str,
            global: bool,
            category: &str,
        ) -> Result<()>;
        fn session_memory_output_path(self: &LibrarySession, assembly_id: &str) -> Result<String>;
        fn session_preserve_assembly_memory(
            self: &LibrarySession,
            assembly_id: &str,
            export_id: i64,
        ) -> Result<()>;
        /// Lists active multi-asset assembly documents.
        fn session_sound_assemblies(self: &LibrarySession)
        -> Result<Vec<SoundAssemblySummaryWire>>;
        /// Creates a sequence (0) or layered (1) assembly from Library assets.
        fn session_create_sound_assembly(
            self: &LibrarySession,
            name: &str,
            asset_ids: &[String],
            layout: u8,
        ) -> Result<SoundAssemblyRevisionWire>;
        /// Reads the newest immutable assembly revision and exact source inputs.
        fn session_sound_assembly(
            self: &LibrarySession,
            assembly_id: &str,
        ) -> Result<SoundAssemblyRevisionWire>;
        /// Pages compact saved project history without resolving or rendering sources.
        fn session_sound_assembly_history(
            self: &LibrarySession,
            assembly_id: &str,
            before_revision_number: u32,
        ) -> Result<Vec<SoundAssemblySummaryWire>>;
        /// Resolves an exact saved version, scoped to the requested project.
        fn session_sound_assembly_at_revision(
            self: &LibrarySession,
            assembly_id: &str,
            revision_id: i64,
        ) -> Result<SoundAssemblyRevisionWire>;
        /// Validates and appends one complete authored document snapshot.
        fn session_save_sound_assembly(
            self: &LibrarySession,
            document_json: &str,
        ) -> Result<SoundAssemblyRevisionWire>;
        /// Archives an assembly without deleting revision or export history.
        fn session_archive_sound_assembly(self: &LibrarySession, assembly_id: &str) -> Result<()>;
        /// Records one verified PCM24 mixdown and generated provenance.
        fn session_record_sound_assembly_export(
            self: &LibrarySession,
            assembly_id: &str,
            assembly_revision_id: i64,
            evidence: &RenderExportWire,
        ) -> Result<i64>;
        /// Lists contextual keyword facets by descending asset count.
        fn session_keyword_facets(self: &LibrarySession) -> Result<Vec<KeywordFacetWire>>;
        /// Lists explainable cross-asset album candidates.
        fn session_smart_albums(self: &LibrarySession) -> Result<Vec<SmartAlbumWire>>;
        /// Lists user-authored albums and explicit membership.
        fn session_user_albums(self: &LibrarySession) -> Result<Vec<UserAlbumWire>>;
        /// Projects bounded Revisit collections at the supplied wall time.
        fn session_revisit_snapshot(
            self: &LibrarySession,
            now_millis: i64,
        ) -> Result<RevisitSnapshotWire>;
        /// Lists named reusable processing recipes at their current revision.
        fn session_processing_recipes(self: &LibrarySession) -> Result<Vec<ProcessingRecipeWire>>;
        /// Lists bounded newest-first processing application history.
        fn session_processing_recipe_history(
            self: &LibrarySession,
        ) -> Result<Vec<ProcessingRecipeHistoryWire>>;
        /// Saves selected processing from one asset-local adjustment revision.
        fn session_create_processing_recipe(
            self: &LibrarySession,
            name: &str,
            source_asset_id: &str,
            components: &[u8],
        ) -> Result<String>;
        /// Renames one active processing recipe.
        fn session_rename_processing_recipe(
            self: &LibrarySession,
            recipe_id: &str,
            name: &str,
        ) -> Result<()>;
        /// Appends selected processing as the recipe's next revision.
        fn session_update_processing_recipe(
            self: &LibrarySession,
            recipe_id: &str,
            source_asset_id: &str,
            components: &[u8],
        ) -> Result<u32>;
        /// Archives a recipe without deleting its audit history.
        fn session_archive_processing_recipe(self: &LibrarySession, recipe_id: &str) -> Result<()>;
        /// Materializes one recipe revision into one or many asset-local graphs.
        fn session_apply_processing_recipe(
            self: &LibrarySession,
            recipe_id: &str,
            target_asset_ids: &[String],
            merge_mode: u8,
        ) -> Result<ProcessingRecipeApplyReceiptWire>;
        /// Safely restores targets still anchored at one application batch.
        fn session_revert_processing_recipe_application(
            self: &LibrarySession,
            batch_id: &str,
        ) -> Result<ProcessingRecipeRevertReceiptWire>;
        /// Creates an empty user album or atomically snapshots a suggestion.
        fn session_create_user_album(
            self: &LibrarySession,
            name: &str,
            member_asset_ids: &[String],
        ) -> Result<i64>;
        /// Renames a user album.
        fn session_rename_user_album(
            self: &LibrarySession,
            album_id: i64,
            name: &str,
        ) -> Result<()>;
        /// Deletes a user album without touching its sounds.
        fn session_delete_user_album(self: &LibrarySession, album_id: i64) -> Result<()>;
        /// Adds or removes one explicit album member.
        fn session_set_user_album_membership(
            self: &LibrarySession,
            album_id: i64,
            asset_id: &str,
            included: bool,
        ) -> Result<bool>;
        /// Total registered asset count.
        fn session_asset_count(self: &LibrarySession) -> u64;
        /// The catalog file path.
        fn session_catalog_path(self: &LibrarySession) -> String;
        /// The cache root path.
        fn session_cache_root(self: &LibrarySession) -> String;
        /// Lists imported IRs after verifying or rebuilding their prepared bytes.
        fn session_impulse_responses(self: &LibrarySession) -> Result<Vec<ImpulseResponseWire>>;
        /// Imports one local WAV with an explicit user rights declaration.
        #[allow(clippy::too_many_arguments)]
        fn session_import_impulse_response(
            self: &LibrarySession,
            source_path: &str,
            display_name: &str,
            creator: &str,
            source_url: &str,
            attribution: &str,
            rights_kind: &str,
            spdx_expression: &str,
            license_url: &str,
        ) -> Result<ImpulseResponseWire>;
        /// Imports one local WAV under an explicit preparation layout.
        #[allow(clippy::too_many_arguments)]
        fn session_import_impulse_response_with_layout(
            self: &LibrarySession,
            source_path: &str,
            preparation_layout: &str,
            display_name: &str,
            creator: &str,
            source_url: &str,
            attribution: &str,
            rights_kind: &str,
            spdx_expression: &str,
            license_url: &str,
        ) -> Result<ImpulseResponseWire>;
        /// Returns the waveform artifact for an asset, building and caching
        /// it when absent.
        fn session_waveform_artifact(
            self: &LibrarySession,
            asset_id: &str,
        ) -> Result<WaveformArtifactWire>;
        /// Returns every transcript evidence record for an asset, newest
        /// first.
        fn session_transcripts(
            self: &LibrarySession,
            asset_id: &str,
        ) -> Result<Vec<TranscriptWire>>;
        /// Returns the persisted hierarchical outline of a long recording.
        fn session_long_audio_chapters(
            self: &LibrarySession,
            asset_id: &str,
        ) -> Result<Vec<LongAudioChapterWire>>;
        /// Stores user-owned Like and rating state for one asset.
        fn session_set_asset_affinity(
            self: &LibrarySession,
            asset_id: &str,
            liked: bool,
            rating: u8,
        ) -> Result<()>;
        /// Appends a user calibration over model-derived descriptive metadata.
        #[allow(clippy::too_many_arguments)]
        fn session_calibrate_asset_metadata(
            self: &LibrarySession,
            asset_id: &str,
            sound_caption: &str,
            summary: &str,
            event_type: &str,
            mood: &str,
            keywords_json: &str,
            transcript_text: &str,
            language: &str,
            calibrated_fields_json: &str,
        ) -> Result<i64>;
        /// Records bounded source-anchored listening progress for one asset.
        fn session_record_listening_progress(
            self: &LibrarySession,
            asset_id: &str,
            position_millis: u64,
            playback_start_millis: u64,
            playback_end_millis: u64,
        ) -> Result<AssetListeningStateWire>;
        /// Appends a validated non-destructive adjustment revision.
        fn session_set_asset_adjustment(
            self: &LibrarySession,
            asset_id: &str,
            adjustment: &AssetAdjustmentWire,
        ) -> Result<()>;
        /// Records provenance after a WAV has been atomically published.
        fn session_record_render_export(
            self: &LibrarySession,
            asset_id: &str,
            adjustment_revision_id: i64,
            evidence: &RenderExportWire,
        ) -> Result<i64>;
        /// Records a delivery rendered from one verified post-effect spectral
        /// working copy and snapshots its mutable manifest into provenance.
        fn session_record_rendered_spectral_working_copy_export(
            self: &LibrarySession,
            asset_id: &str,
            adjustment_revision_id: i64,
            working_copy_id: i64,
            rendered_source_path: &str,
            evidence: &RenderExportWire,
        ) -> Result<i64>;
        /// Moves one completed private render into the content-addressed cache
        /// and atomically records it as a frozen spectral working-copy parent.
        fn session_create_rendered_spectral_working_copy(
            self: &LibrarySession,
            asset_id: &str,
            adjustment_revision_id: i64,
            rendered_path: &str,
        ) -> Result<RenderedSpectralWorkingCopyWire>;
        /// Lists preserved post-effect working copies and cache-readable paths.
        fn session_rendered_spectral_working_copies(
            self: &LibrarySession,
            asset_id: &str,
        ) -> Result<Vec<RenderedSpectralWorkingCopyWire>>;
        /// Bypasses or restores one whole post-effect working copy.
        fn session_set_rendered_spectral_working_copy_enabled(
            self: &LibrarySession,
            asset_id: &str,
            working_copy_id: i64,
            enabled: bool,
        ) -> Result<()>;
        /// Publishes the next current cache identity after an accepted
        /// deterministic erase into the one shared working copy.
        fn session_commit_rendered_spectral_erase(
            self: &LibrarySession,
            asset_id: &str,
            working_copy_id: i64,
            rendered_path: &str,
            start_millis: u64,
            end_millis: u64,
            low_hertz: u16,
            high_hertz: u16,
            attenuation_centibels: i16,
            time_feather_millis: u16,
            frequency_feather_hertz: u16,
        ) -> Result<RenderedSpectralWorkingCopyWire>;
        /// Removes one whole post-effect working copy; cache bytes remain
        /// disposable and are never user-authored state.
        fn session_remove_rendered_spectral_working_copy(
            self: &LibrarySession,
            asset_id: &str,
            working_copy_id: i64,
        ) -> Result<()>;
        /// Starts the background worker pool (idempotent).
        fn session_start_workers(self: &LibrarySession, runtime_endpoint: &str) -> Result<()>;
        /// Reads the process-local worker transition revision.
        fn session_worker_state_revision(self: &LibrarySession) -> u64;
        /// Returns the current analysis stage for one asset.
        fn session_analysis_status(
            self: &LibrarySession,
            asset_id: &str,
        ) -> Result<AnalysisStatusWire>;
        /// Returns lightweight analysis status for every Library asset.
        fn session_analysis_statuses(self: &LibrarySession) -> Result<Vec<AnalysisStatusWire>>;
        /// Requeues the failed analysis stage for one asset.
        fn session_retry_analysis(self: &LibrarySession, asset_id: &str) -> Result<()>;
        /// Requeues every current failed stage that requires manual recovery.
        fn session_retry_failed_analysis(self: &LibrarySession) -> Result<u64>;
        /// Queues scans for every enabled root (incremental detection).
        fn session_queue_scans(self: &LibrarySession) -> Result<u64>;
        /// Reads aggregate job statistics.
        fn session_job_stats(self: &LibrarySession) -> Result<JobStatsWire>;
        /// Lists configured scan roots.
        fn session_list_roots(self: &LibrarySession) -> Result<Vec<ScanRootWire>>;
        /// Adds a scan root and queues its scan.
        fn session_add_root(self: &LibrarySession, root: &str) -> Result<()>;
        /// Removes a scan root by id.
        fn session_remove_root(self: &LibrarySession, id: i64) -> Result<()>;
        /// Full-text search over indexed transcripts.
        fn session_search(
            self: &LibrarySession,
            query: &str,
            limit: u64,
        ) -> Result<Vec<SearchHitWire>>;
    }
}

fn infer_runtime_credential_available() -> bool {
    echo_core::infer_runtime_credential_available()
}

fn semantic_search_catalog(
    catalog_path: &str,
    runtime_endpoint: &str,
    query: &str,
    limit: u64,
) -> Result<Vec<ffi::SemanticSearchHitWire>, String> {
    let catalog = echo_catalog::open_catalog(std::path::Path::new(catalog_path))
        .map_err(|error| error.to_string())?;
    let credential_path =
        echo_core::infer_runtime_credential_path().map_err(|error| error.to_string())?;
    echo_core::semantic_search(
        &catalog,
        &echo_core::InferRuntimeConfig {
            base_url: runtime_endpoint.to_owned(),
            credential_path,
        },
        query,
        limit,
    )
    .map(|hits| {
        hits.into_iter()
            .map(|hit| ffi::SemanticSearchHitWire {
                asset_id: hit.asset_id,
                score: hit.score,
            })
            .collect()
    })
    .map_err(|error| error.to_string())
}

/// Opens (creating if needed) the catalog at `path` with the cache root at
/// `cache_root`.
///
/// # Errors
///
/// Returns the session error message when the catalog cannot be opened.
pub fn audio_file_patterns() -> String {
    echo_core::audio_file_patterns()
}

pub fn open_session(path: &str, cache_root: &str) -> Result<Box<LibrarySession>, String> {
    session::open_session(path, cache_root)
        .map(Box::new)
        .map_err(|error| error.message)
}

/// Loads or builds one display-only spectrogram through an independent catalog
/// attachment. The catalog serializes its own access, while this keeps an
/// expensive cold-cache decode outside the Qt main thread.
///
/// # Errors
///
/// Returns the session error message when the catalog or source cannot be
/// opened, or the derived artifact is unavailable.
pub fn spectrogram_artifact_for_catalog(
    catalog_path: &str,
    cache_root: &str,
    asset_id: &str,
) -> Result<ffi::SpectrogramArtifactWire, String> {
    session::open_session(catalog_path, cache_root)
        .and_then(|session| session.spectrogram_artifact(asset_id))
        .map(|payload| ffi::SpectrogramArtifactWire {
            canonical_sample_rate: payload.canonical_sample_rate,
            window_frames: payload.window_frames,
            hop_frames: payload.hop_frames,
            time_columns: payload.time_columns,
            frequency_bins: payload.frequency_bins,
            magnitudes: payload.magnitudes,
        })
        .map_err(|error| error.message)
}

/// Builds a display-only spectrogram from an already verified private render
/// cache path. The resulting pixels are ephemeral and never become Catalog
/// editing evidence.
pub fn spectrogram_artifact_for_path(path: &str) -> Result<ffi::SpectrogramArtifactWire, String> {
    echo_bridge::spectrogram::build_spectrogram_overview(Path::new(path), 1024, 128)
        .map(|payload| ffi::SpectrogramArtifactWire {
            canonical_sample_rate: payload.canonical_sample_rate,
            window_frames: payload.window_frames,
            hop_frames: payload.hop_frames,
            time_columns: payload.time_columns,
            frequency_bins: payload.frequency_bins,
            magnitudes: payload.magnitudes,
        })
        .map_err(|error| error.message)
}

impl LibrarySession {
    fn session_save_project_clip_adjustment(
        &self,
        document_json: &str,
        clip_id: &str,
        asset_id: &str,
        adjustment: &ffi::AssetAdjustmentWire,
    ) -> Result<ffi::SoundAssemblyRevisionWire, String> {
        self.save_project_clip_adjustment(document_json, clip_id, asset_id, adjustment)
            .map_err(|e| e.to_string())
    }
    fn session_export_source_disclosure(
        &self,
        id: &str,
        assembly_revision: i64,
    ) -> Result<String, String> {
        self.export_source_disclosure(id, assembly_revision)
            .map_err(|e| e.message)
    }

    fn session_set_source_disclosure(
        &self,
        id: &str,
        expected_revision: i64,
        spans_json: &str,
    ) -> Result<(), String> {
        self.set_source_disclosure(id, expected_revision, spans_json)
            .map_err(|e| e.to_string())
    }
    fn session_set_sound_membership(
        &self,
        id: &str,
        memory: bool,
        materials: bool,
        category: &str,
    ) -> Result<(), String> {
        self.set_sound_membership(id, memory, materials, category)
            .map_err(|e| e.to_string())
    }
    fn session_project_materials(&self, id: &str) -> Result<Vec<String>, String> {
        self.project_materials(id).map_err(|e| e.to_string())
    }
    fn session_queue_material_import(
        &self,
        path: &str,
        id: &str,
        global: bool,
        category: &str,
    ) -> Result<(), String> {
        self.queue_material_import(path, id, global, category)
            .map_err(|e| e.to_string())
    }
    fn session_memory_output_path(&self, id: &str) -> Result<String, String> {
        self.memory_output_path(id).map_err(|e| e.to_string())
    }
    fn session_preserve_assembly_memory(&self, id: &str, export_id: i64) -> Result<(), String> {
        self.preserve_assembly_memory(id, export_id)
            .map_err(|e| e.to_string())
    }

    /// Lists registered assets, newest import first.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the catalog read fails.
    fn session_list_originals(&self) -> Result<Vec<ffi::AssetSummaryWire>, String> {
        self.list_originals().map_err(|e| e.to_string())
    }
    fn session_list_assets(&self) -> Result<Vec<ffi::AssetSummaryWire>, String> {
        self.list_assets().map_err(|error| error.message)
    }

    fn session_sound_assemblies(&self) -> Result<Vec<ffi::SoundAssemblySummaryWire>, String> {
        self.sound_assemblies().map_err(|error| error.message)
    }

    fn session_create_sound_assembly(
        &self,
        name: &str,
        asset_ids: &[String],
        layout: u8,
    ) -> Result<ffi::SoundAssemblyRevisionWire, String> {
        self.create_sound_assembly(name, asset_ids, layout)
            .map_err(|error| error.message)
    }

    fn session_sound_assembly(
        &self,
        assembly_id: &str,
    ) -> Result<ffi::SoundAssemblyRevisionWire, String> {
        self.sound_assembly(assembly_id)
            .map_err(|error| error.message)
    }

    fn session_sound_assembly_history(
        &self,
        assembly_id: &str,
        before_revision_number: u32,
    ) -> Result<Vec<ffi::SoundAssemblySummaryWire>, String> {
        self.sound_assembly_history(assembly_id, before_revision_number)
            .map_err(|error| error.message)
    }
    fn session_sound_assembly_at_revision(
        &self,
        assembly_id: &str,
        revision_id: i64,
    ) -> Result<ffi::SoundAssemblyRevisionWire, String> {
        self.sound_assembly_at_revision(assembly_id, revision_id)
            .map_err(|error| error.message)
    }

    fn session_save_sound_assembly(
        &self,
        document_json: &str,
    ) -> Result<ffi::SoundAssemblyRevisionWire, String> {
        self.save_sound_assembly(document_json)
            .map_err(|error| error.message)
    }

    fn session_archive_sound_assembly(&self, assembly_id: &str) -> Result<(), String> {
        self.archive_sound_assembly(assembly_id)
            .map_err(|error| error.message)
    }

    fn session_record_sound_assembly_export(
        &self,
        assembly_id: &str,
        assembly_revision_id: i64,
        evidence: &ffi::RenderExportWire,
    ) -> Result<i64, String> {
        self.record_sound_assembly_export(assembly_id, assembly_revision_id, evidence)
            .map_err(|error| error.message)
    }

    /// Lists contextual keyword facets by descending asset count.
    fn session_keyword_facets(&self) -> Result<Vec<ffi::KeywordFacetWire>, String> {
        self.keyword_facets().map_err(|error| error.message)
    }

    /// Lists explainable cross-asset album candidates.
    fn session_smart_albums(&self) -> Result<Vec<ffi::SmartAlbumWire>, String> {
        self.smart_albums().map_err(|error| error.message)
    }

    /// Lists user-authored albums.
    fn session_user_albums(&self) -> Result<Vec<ffi::UserAlbumWire>, String> {
        self.user_albums().map_err(|error| error.message)
    }

    /// Projects bounded source-anchored collections for Revisit.
    fn session_revisit_snapshot(
        &self,
        now_millis: i64,
    ) -> Result<ffi::RevisitSnapshotWire, String> {
        self.revisit_snapshot(now_millis)
            .map_err(|error| error.message)
    }

    /// Lists current named processing recipe revisions.
    fn session_processing_recipes(&self) -> Result<Vec<ffi::ProcessingRecipeWire>, String> {
        self.processing_recipes().map_err(|error| error.message)
    }

    /// Lists bounded durable processing application and revert history.
    fn session_processing_recipe_history(
        &self,
    ) -> Result<Vec<ffi::ProcessingRecipeHistoryWire>, String> {
        self.processing_recipe_history()
            .map_err(|error| error.message)
    }

    /// Saves a reusable recipe from one persisted asset adjustment.
    fn session_create_processing_recipe(
        &self,
        name: &str,
        source_asset_id: &str,
        components: &[u8],
    ) -> Result<String, String> {
        self.create_processing_recipe(name, source_asset_id, components)
            .map_err(|error| error.message)
    }

    /// Renames one active processing recipe.
    fn session_rename_processing_recipe(&self, recipe_id: &str, name: &str) -> Result<(), String> {
        self.rename_processing_recipe(recipe_id, name)
            .map_err(|error| error.message)
    }

    /// Appends one immutable recipe revision from a saved sound adjustment.
    fn session_update_processing_recipe(
        &self,
        recipe_id: &str,
        source_asset_id: &str,
        components: &[u8],
    ) -> Result<u32, String> {
        self.update_processing_recipe(recipe_id, source_asset_id, components)
            .map_err(|error| error.message)
    }

    /// Archives one recipe while preserving revisions and batch receipts.
    fn session_archive_processing_recipe(&self, recipe_id: &str) -> Result<(), String> {
        self.archive_processing_recipe(recipe_id)
            .map_err(|error| error.message)
    }

    /// Applies one current immutable recipe revision to explicit targets.
    fn session_apply_processing_recipe(
        &self,
        recipe_id: &str,
        target_asset_ids: &[String],
        merge_mode: u8,
    ) -> Result<ffi::ProcessingRecipeApplyReceiptWire, String> {
        self.apply_processing_recipe(recipe_id, target_asset_ids, merge_mode)
            .map_err(|error| error.message)
    }

    /// Safely reverts one processing-recipe application batch.
    fn session_revert_processing_recipe_application(
        &self,
        batch_id: &str,
    ) -> Result<ffi::ProcessingRecipeRevertReceiptWire, String> {
        self.revert_processing_recipe_application(batch_id)
            .map_err(|error| error.message)
    }

    /// Creates one user album with an optional member snapshot.
    fn session_create_user_album(
        &self,
        name: &str,
        member_asset_ids: &[String],
    ) -> Result<i64, String> {
        self.create_user_album(name, member_asset_ids)
            .map_err(|error| error.message)
    }

    /// Renames one user album.
    fn session_rename_user_album(&self, album_id: i64, name: &str) -> Result<(), String> {
        self.rename_user_album(album_id, name)
            .map_err(|error| error.message)
    }

    /// Deletes one user album.
    fn session_delete_user_album(&self, album_id: i64) -> Result<(), String> {
        self.delete_user_album(album_id)
            .map_err(|error| error.message)
    }

    /// Adds or removes one explicit album member.
    fn session_set_user_album_membership(
        &self,
        album_id: i64,
        asset_id: &str,
        included: bool,
    ) -> Result<bool, String> {
        self.set_user_album_membership(album_id, asset_id, included)
            .map_err(|error| error.message)
    }

    /// Total registered asset count.
    fn session_asset_count(&self) -> u64 {
        self.asset_count()
    }

    /// The catalog file path.
    fn session_catalog_path(&self) -> String {
        self.catalog_path().to_string_lossy().into_owned()
    }

    /// The cache root path.
    fn session_cache_root(&self) -> String {
        self.cache_root().to_string_lossy().into_owned()
    }

    fn session_impulse_responses(&self) -> Result<Vec<ffi::ImpulseResponseWire>, String> {
        self.impulse_responses().map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    fn session_import_impulse_response(
        &self,
        source_path: &str,
        display_name: &str,
        creator: &str,
        source_url: &str,
        attribution: &str,
        rights_kind: &str,
        spdx_expression: &str,
        license_url: &str,
    ) -> Result<ffi::ImpulseResponseWire, String> {
        self.import_impulse_response(
            source_path,
            display_name,
            creator,
            source_url,
            attribution,
            rights_kind,
            spdx_expression,
            license_url,
        )
        .map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    fn session_import_impulse_response_with_layout(
        &self,
        source_path: &str,
        preparation_layout: &str,
        display_name: &str,
        creator: &str,
        source_url: &str,
        attribution: &str,
        rights_kind: &str,
        spdx_expression: &str,
        license_url: &str,
    ) -> Result<ffi::ImpulseResponseWire, String> {
        self.import_impulse_response_with_layout(
            source_path,
            preparation_layout,
            display_name,
            creator,
            source_url,
            attribution,
            rights_kind,
            spdx_expression,
            license_url,
        )
        .map_err(|error| error.to_string())
    }

    /// Returns the waveform artifact for an asset, building and caching it
    /// when absent.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the artifact cannot be built,
    /// read, or decoded.
    fn session_waveform_artifact(
        &self,
        asset_id: &str,
    ) -> Result<ffi::WaveformArtifactWire, String> {
        self.waveform_artifact(asset_id)
            .map_err(|error| error.message)
    }

    /// Returns every transcript evidence record for an asset, newest first.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the catalog read fails.
    fn session_transcripts(&self, asset_id: &str) -> Result<Vec<ffi::TranscriptWire>, String> {
        self.transcripts(asset_id).map_err(|error| error.message)
    }

    fn session_long_audio_chapters(
        &self,
        asset_id: &str,
    ) -> Result<Vec<ffi::LongAudioChapterWire>, String> {
        self.long_audio_chapters(asset_id)
            .map_err(|error| error.message)
    }

    /// Stores user-owned Like and rating state for one asset.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the asset identity or write is
    /// invalid.
    fn session_set_asset_affinity(
        &self,
        asset_id: &str,
        liked: bool,
        rating: u8,
    ) -> Result<(), String> {
        self.set_asset_affinity(asset_id, liked, rating)
            .map_err(|error| error.message)
    }

    #[allow(clippy::too_many_arguments)]
    fn session_calibrate_asset_metadata(
        &self,
        asset_id: &str,
        sound_caption: &str,
        summary: &str,
        event_type: &str,
        mood: &str,
        keywords_json: &str,
        transcript_text: &str,
        language: &str,
        calibrated_fields_json: &str,
    ) -> Result<i64, String> {
        self.calibrate_asset_metadata(
            asset_id,
            sound_caption,
            summary,
            event_type,
            mood,
            keywords_json,
            transcript_text,
            language,
            calibrated_fields_json,
        )
        .map_err(|error| error.message)
    }

    /// Records one user-owned listening checkpoint.
    fn session_record_listening_progress(
        &self,
        asset_id: &str,
        position_millis: u64,
        playback_start_millis: u64,
        playback_end_millis: u64,
    ) -> Result<ffi::AssetListeningStateWire, String> {
        self.record_listening_progress(
            asset_id,
            position_millis,
            playback_start_millis,
            playback_end_millis,
        )
        .map_err(|error| error.message)
    }

    /// Appends one non-destructive adjustment revision.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the graph or asset identity is
    /// invalid.
    fn session_set_asset_adjustment(
        &self,
        asset_id: &str,
        adjustment: &ffi::AssetAdjustmentWire,
    ) -> Result<(), String> {
        self.set_asset_adjustment(asset_id, adjustment)
            .map_err(|error| error.message)
    }

    /// Records one completed render publication and its source revision.
    fn session_record_render_export(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        evidence: &ffi::RenderExportWire,
    ) -> Result<i64, String> {
        self.record_render_export(
            asset_id,
            adjustment_revision_id,
            &evidence.output_path,
            &evidence.format,
            evidence.sample_rate,
            evidence.channel_count,
            evidence.bit_depth,
            evidence.frame_count,
            evidence.size_bytes,
            evidence.integrated_lufs,
            evidence.true_peak_dbtp,
            &evidence.source_disclosure_comment,
        )
        .map_err(|error| error.message)
    }

    fn session_record_rendered_spectral_working_copy_export(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        working_copy_id: i64,
        rendered_source_path: &str,
        evidence: &ffi::RenderExportWire,
    ) -> Result<i64, String> {
        self.record_rendered_spectral_working_copy_export(
            asset_id,
            adjustment_revision_id,
            working_copy_id,
            rendered_source_path,
            &evidence.output_path,
            &evidence.format,
            evidence.sample_rate,
            evidence.channel_count,
            evidence.bit_depth,
            evidence.frame_count,
            evidence.size_bytes,
            evidence.integrated_lufs,
            evidence.true_peak_dbtp,
            &evidence.source_disclosure_comment,
        )
        .map_err(|error| error.message)
    }

    fn session_create_rendered_spectral_working_copy(
        &self,
        asset_id: &str,
        adjustment_revision_id: i64,
        rendered_path: &str,
    ) -> Result<ffi::RenderedSpectralWorkingCopyWire, String> {
        self.create_rendered_spectral_working_copy(asset_id, adjustment_revision_id, rendered_path)
            .map(session::rendered_spectral_working_copy_wire)
            .map_err(|error| error.message)
    }

    fn session_rendered_spectral_working_copies(
        &self,
        asset_id: &str,
    ) -> Result<Vec<ffi::RenderedSpectralWorkingCopyWire>, String> {
        self.rendered_spectral_working_copies(asset_id)
            .map(|copies| {
                copies
                    .into_iter()
                    .map(session::rendered_spectral_working_copy_wire)
                    .collect()
            })
            .map_err(|error| error.message)
    }

    fn session_set_rendered_spectral_working_copy_enabled(
        &self,
        asset_id: &str,
        working_copy_id: i64,
        enabled: bool,
    ) -> Result<(), String> {
        self.set_rendered_spectral_working_copy_enabled(asset_id, working_copy_id, enabled)
            .map_err(|error| error.message)
    }

    #[allow(clippy::too_many_arguments)]
    fn session_commit_rendered_spectral_erase(
        &self,
        asset_id: &str,
        working_copy_id: i64,
        rendered_path: &str,
        start_millis: u64,
        end_millis: u64,
        low_hertz: u16,
        high_hertz: u16,
        attenuation_centibels: i16,
        time_feather_millis: u16,
        frequency_feather_hertz: u16,
    ) -> Result<ffi::RenderedSpectralWorkingCopyWire, String> {
        self.commit_rendered_spectral_erase(
            asset_id,
            working_copy_id,
            rendered_path,
            start_millis,
            end_millis,
            low_hertz,
            high_hertz,
            attenuation_centibels,
            time_feather_millis,
            frequency_feather_hertz,
        )
        .map(session::rendered_spectral_working_copy_wire)
        .map_err(|error| error.message)
    }

    fn session_remove_rendered_spectral_working_copy(
        &self,
        asset_id: &str,
        working_copy_id: i64,
    ) -> Result<(), String> {
        self.remove_rendered_spectral_working_copy(asset_id, working_copy_id)
            .map_err(|error| error.message)
    }
}

impl LibrarySession {
    /// Starts the background worker pool (idempotent).
    ///
    /// # Errors
    ///
    /// Returns the session error message when the pool cannot start.
    fn session_start_workers(&self, runtime_endpoint: &str) -> Result<(), String> {
        self.start_workers(runtime_endpoint)
            .map_err(|error| error.message)
    }

    /// Returns a cheap invalidation identity for desktop status projection.
    fn session_worker_state_revision(&self) -> u64 {
        self.worker_state_revision()
    }

    /// Returns the current analysis stage for one asset.
    fn session_analysis_status(&self, asset_id: &str) -> Result<ffi::AnalysisStatusWire, String> {
        self.analysis_status(asset_id)
            .map_err(|error| error.message)
    }

    /// Returns lightweight analysis state for Library cards and filters.
    fn session_analysis_statuses(&self) -> Result<Vec<ffi::AnalysisStatusWire>, String> {
        self.analysis_statuses().map_err(|error| error.message)
    }

    /// Requeues the failed analysis stage for one asset.
    fn session_retry_analysis(&self, asset_id: &str) -> Result<(), String> {
        self.retry_analysis(asset_id).map_err(|error| error.message)
    }

    /// Requeues current manually recoverable analysis stages.
    fn session_retry_failed_analysis(&self) -> Result<u64, String> {
        self.retry_failed_analysis().map_err(|error| error.message)
    }

    /// Queues scans for every enabled root (incremental detection).
    ///
    /// # Errors
    ///
    /// Returns the session error message when queueing fails.
    fn session_queue_scans(&self) -> Result<u64, String> {
        self.queue_scans().map_err(|error| error.message)
    }

    /// Reads aggregate job statistics.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the read fails.
    fn session_job_stats(&self) -> Result<ffi::JobStatsWire, String> {
        self.job_stats().map_err(|error| error.message)
    }

    /// Lists configured scan roots.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the read fails.
    fn session_list_roots(&self) -> Result<Vec<ffi::ScanRootWire>, String> {
        self.list_roots().map_err(|error| error.message)
    }

    /// Adds a scan root and queues its scan.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the write fails.
    fn session_add_root(&self, root: &str) -> Result<(), String> {
        self.add_root(root).map_err(|error| error.message)
    }

    /// Removes a scan root by id.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the write fails.
    fn session_remove_root(&self, id: i64) -> Result<(), String> {
        self.remove_root(id).map_err(|error| error.message)
    }

    /// Full-text search over indexed transcripts.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the search fails.
    fn session_search(&self, query: &str, limit: u64) -> Result<Vec<ffi::SearchHitWire>, String> {
        self.search(query, limit).map_err(|error| error.message)
    }
}

/// Opens a private editor store without any Library workers.
///
/// # Errors
/// Returns a diagnostic when validation or project I/O fails.
pub fn open_editor_session(root: &str) -> Result<Box<LibrarySession>, String> {
    editor_session::open(Path::new(root)).map(Box::new)
}
/// Admits one source explicitly, without background analysis.
///
/// # Errors
/// Returns a diagnostic when validation or project I/O fails.
pub fn editor_import_audio(root: &str, input: &str) -> Result<String, String> {
    editor_session::import(Path::new(root), Path::new(input))
}
/// Atomically writes a portable editing project.
///
/// # Errors
/// Returns a diagnostic when validation or project I/O fails.
pub fn editor_save_project(root: &str, destination: &str) -> Result<(), String> {
    editor_project::save(Path::new(root), Path::new(destination))
}
/// Validates and extracts a project into an empty private working directory.
///
/// # Errors
/// Returns a diagnostic when validation or project I/O fails.
pub fn editor_open_project(project: &str, root: &str) -> Result<(), String> {
    editor_project::open(Path::new(project), Path::new(root))
}
impl LibrarySession {
    fn session_is_independent(&self) -> bool {
        self.independent
    }
}

fn transcribe_editor_selection(
    catalog: &str,
    cache: &str,
    id: &str,
    start: u64,
    end: u64,
    endpoint: &str,
) -> Result<String, String> {
    selection_transcription::transcribe(catalog, cache, id, start, end, endpoint)
}
impl LibrarySession {
    fn session_accept_selection_transcript(&self, id: &str, value: &str) -> Result<(), String> {
        self.accept_selection_transcript(id, value)
    }
    fn session_selection_transcripts(&self, id: &str) -> Result<String, String> {
        self.selection_transcripts(id)
    }
}

fn generate_narration_candidate(
    text: &str,
    directory: &str,
    endpoint: &str,
) -> Result<Box<NarrationCandidate>, String> {
    generated_narration::generate(text, directory, endpoint)
}
fn narration_candidate_details(candidate: &NarrationCandidate) -> Result<String, String> {
    candidate.details_json().map_err(|e| e.to_string())
}
fn accept_narration_candidate(
    catalog: &str,
    candidate: &NarrationCandidate,
    assembly: &str,
    global: bool,
) -> Result<String, String> {
    generated_narration::accept(catalog, candidate, assembly, global)
}

#[cfg(test)]
mod tests;
