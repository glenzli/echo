//! Long-lived desktop services bridging Qt to the Rust memory engine.
//!
//! QML and desktop controllers never open `SQLite`, call `FFmpeg`, or interpret
//! cache paths: they talk to this crate through the generated CXX ABI, and it
//! owns the durable [`LibrarySession`] lifecycle.

mod session;

use crate::session::LibrarySession;

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

    /// Bounded presentation projection of one asset for the desktop shell.
    #[derive(Debug)]
    struct AssetSummaryWire {
        id: String,
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
        liked: bool,
        rating: u8,
        adjustment_revision: i64,
        trim_start_millis: u64,
        trim_end_millis: u64,
        fade_in_millis: u64,
        fade_out_millis: u64,
        fade_in_curve: u8,
        fade_out_curve: u8,
        gain_centibels: i16,
        low_cut_hertz: u16,
        equalizer_bands: Vec<EqualizerBandWire>,
        compressor_enabled: bool,
        compressor_threshold_centibels: i16,
        compressor_ratio_tenths: u16,
        compressor_attack_millis: u16,
        compressor_release_millis: u16,
        compressor_makeup_centibels: i16,
        limiter_enabled: bool,
        limiter_ceiling_centibels: i16,
        limiter_release_millis: u16,
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
        equalizer_bands: Vec<EqualizerBandWire>,
        compressor_enabled: bool,
        compressor_threshold_centibels: i16,
        compressor_ratio_tenths: u16,
        compressor_attack_millis: u16,
        compressor_release_millis: u16,
        compressor_makeup_centibels: i16,
        limiter_enabled: bool,
        limiter_ceiling_centibels: i16,
        limiter_release_millis: u16,
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
        stage: String,
        state: String,
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

    extern "Rust" {
        type LibrarySession;

        /// Reports whether Echo's owner-only Runtime credential is available.
        fn infer_runtime_credential_available() -> bool;
        /// Opens (creating if needed) the catalog and cache at the given
        /// roots.
        fn open_session(path: &str, cache_root: &str) -> Result<Box<LibrarySession>>;
        /// Lists registered assets, newest import first.
        fn session_list_assets(self: &LibrarySession) -> Result<Vec<AssetSummaryWire>>;
        /// Lists contextual keyword facets by descending asset count.
        fn session_keyword_facets(self: &LibrarySession) -> Result<Vec<KeywordFacetWire>>;
        /// Lists explainable cross-asset album candidates.
        fn session_smart_albums(self: &LibrarySession) -> Result<Vec<SmartAlbumWire>>;
        /// Total registered asset count.
        fn session_asset_count(self: &LibrarySession) -> u64;
        /// The catalog file path.
        fn session_catalog_path(self: &LibrarySession) -> String;
        /// The cache root path.
        fn session_cache_root(self: &LibrarySession) -> String;
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
        /// Stores user-owned Like and rating state for one asset.
        fn session_set_asset_affinity(
            self: &LibrarySession,
            asset_id: &str,
            liked: bool,
            rating: u8,
        ) -> Result<()>;
        /// Appends a validated non-destructive adjustment revision.
        fn session_set_asset_adjustment(
            self: &LibrarySession,
            asset_id: &str,
            adjustment: &AssetAdjustmentWire,
        ) -> Result<()>;
        /// Starts the background worker pool (idempotent).
        fn session_start_workers(self: &LibrarySession, runtime_endpoint: &str) -> Result<()>;
        /// Returns the current analysis stage for one asset.
        fn session_analysis_status(
            self: &LibrarySession,
            asset_id: &str,
        ) -> Result<AnalysisStatusWire>;
        /// Requeues the failed analysis stage for one asset.
        fn session_retry_analysis(self: &LibrarySession, asset_id: &str) -> Result<()>;
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

/// Opens (creating if needed) the catalog at `path` with the cache root at
/// `cache_root`.
///
/// # Errors
///
/// Returns the session error message when the catalog cannot be opened.
pub fn open_session(path: &str, cache_root: &str) -> Result<Box<LibrarySession>, String> {
    session::open_session(path, cache_root)
        .map(Box::new)
        .map_err(|error| error.message)
}

impl LibrarySession {
    /// Lists registered assets, newest import first.
    ///
    /// # Errors
    ///
    /// Returns the session error message when the catalog read fails.
    fn session_list_assets(&self) -> Result<Vec<ffi::AssetSummaryWire>, String> {
        self.list_assets().map_err(|error| error.message)
    }

    /// Lists contextual keyword facets by descending asset count.
    fn session_keyword_facets(&self) -> Result<Vec<ffi::KeywordFacetWire>, String> {
        self.keyword_facets().map_err(|error| error.message)
    }

    /// Lists explainable cross-asset album candidates.
    fn session_smart_albums(&self) -> Result<Vec<ffi::SmartAlbumWire>, String> {
        self.smart_albums().map_err(|error| error.message)
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

    /// Returns the current analysis stage for one asset.
    fn session_analysis_status(&self, asset_id: &str) -> Result<ffi::AnalysisStatusWire, String> {
        self.analysis_status(asset_id)
            .map_err(|error| error.message)
    }

    /// Requeues the failed analysis stage for one asset.
    fn session_retry_analysis(&self, asset_id: &str) -> Result<(), String> {
        self.retry_analysis(asset_id).map_err(|error| error.message)
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

#[cfg(test)]
mod tests;
