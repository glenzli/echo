//! Long-lived desktop services bridging Qt to the Rust memory engine.
//!
//! QML and desktop controllers never open `SQLite`, call `FFmpeg`, or interpret
//! cache paths: they talk to this crate through the generated CXX ABI, and it
//! owns the durable [`LibrarySession`] lifecycle.

mod session;

use crate::session::LibrarySession;

#[cxx::bridge(namespace = "echo::desktop")]
mod ffi {
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
        summary: String,
        event_type: String,
        mood: String,
        keywords: Vec<String>,
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

        /// Opens (creating if needed) the catalog and cache at the given
        /// roots.
        fn open_session(path: &str, cache_root: &str) -> Result<Box<LibrarySession>>;
        /// Lists registered assets, newest import first.
        fn session_list_assets(self: &LibrarySession) -> Result<Vec<AssetSummaryWire>>;
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
        /// Transcribes an asset with the configured MLX worker. Stateless so
        /// it can run on a background thread.
        fn transcribe_asset(
            catalog_path: &str,
            asset_id: &str,
            model_root: &str,
            python: &str,
            worker_script: &str,
        ) -> Result<u32>;
        /// Starts the background worker pool (idempotent).
        fn session_start_workers(
            self: &LibrarySession,
            model_root: &str,
            python: &str,
            worker_script: &str,
            ollama_endpoint: &str,
            ollama_model: &str,
        ) -> Result<()>;
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
}

/// Transcribes an asset with the configured MLX worker. Stateless so it can
/// run on a background thread.
///
/// # Errors
///
/// Returns the session error message when the model is missing or the worker
/// fails.
pub fn transcribe_asset(
    catalog_path: &str,
    asset_id: &str,
    model_root: &str,
    python: &str,
    worker_script: &str,
) -> Result<u32, String> {
    session::transcribe_asset(catalog_path, asset_id, model_root, python, worker_script)
        .map_err(|error| error.message)
}

impl LibrarySession {
    /// Starts the background worker pool (idempotent).
    ///
    /// # Errors
    ///
    /// Returns the session error message when the pool cannot start.
    fn session_start_workers(
        &self,
        model_root: &str,
        python: &str,
        worker_script: &str,
        ollama_endpoint: &str,
        ollama_model: &str,
    ) -> Result<(), String> {
        self.start_workers(
            model_root,
            python,
            worker_script,
            ollama_endpoint,
            ollama_model,
        )
        .map_err(|error| error.message)
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
