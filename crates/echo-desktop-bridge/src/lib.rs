//! Long-lived desktop services bridging Qt to the Rust memory engine.
//!
//! QML and desktop controllers never open SQLite, call FFmpeg, or interpret
//! cache paths: they talk to this crate through the generated CXX ABI, and it
//! owns the durable [`LibrarySession`] lifecycle.

mod session;

use crate::session::LibrarySession;

#[cxx::bridge(namespace = "echo::desktop")]
mod ffi {
    /// Bounded presentation projection of one asset for the desktop shell.
    struct AssetSummaryWire {
        id: String,
        path: String,
        codec: String,
        duration_millis: u64,
        imported_at_millis: i64,
        max_level: u8,
    }

    /// One pyramid level of a cached waveform artifact.
    struct WaveformLevelWire {
        samples_per_bucket: u32,
        mins: Vec<f32>,
        maxs: Vec<f32>,
    }

    /// A cached waveform artifact for display.
    struct WaveformArtifactWire {
        canonical_sample_rate: u32,
        levels: Vec<WaveformLevelWire>,
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
}
