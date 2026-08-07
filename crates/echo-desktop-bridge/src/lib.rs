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

    extern "Rust" {
        type LibrarySession;

        /// Opens (creating if needed) the catalog at `path`.
        fn open_session(path: &str) -> Result<Box<LibrarySession>>;
        /// Lists registered assets, newest import first.
        fn session_list_assets(self: &LibrarySession) -> Result<Vec<AssetSummaryWire>>;
        /// Total registered asset count.
        fn session_asset_count(self: &LibrarySession) -> u64;
        /// The catalog file path.
        fn session_catalog_path(self: &LibrarySession) -> String;
    }
}

/// Opens (creating if needed) the catalog at `path`.
///
/// # Errors
///
/// Returns the session error message when the catalog cannot be opened.
pub fn open_session(path: &str) -> Result<Box<LibrarySession>, String> {
    session::open_session(path)
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
}
