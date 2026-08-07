//! The long-lived Library session: one catalog attachment for the desktop
//! process lifetime.

use std::path::{Path, PathBuf};

use echo_catalog::{Catalog, list_assets, open_catalog};

use crate::ffi::AssetSummaryWire;

/// One catalog attachment. Sessions are created on the Qt main thread and
/// reused; the catalog serializes its own writes.
pub struct LibrarySession {
    catalog: Catalog,
    catalog_path: PathBuf,
}

/// Error vocabulary for session operations.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct SessionError {
    pub message: String,
}

/// Opens (creating if needed) the catalog at `path`.
///
/// # Errors
///
/// Returns [`SessionError`] when the catalog cannot be opened.
pub fn open_session(path: &str) -> Result<LibrarySession, SessionError> {
    let catalog_path = PathBuf::from(path);
    let catalog = open_catalog(&catalog_path).map_err(|error| SessionError {
        message: error.to_string(),
    })?;
    Ok(LibrarySession {
        catalog,
        catalog_path,
    })
}

impl LibrarySession {
    /// Lists registered assets, newest import first.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the catalog read fails.
    pub fn list_assets(&self) -> Result<Vec<AssetSummaryWire>, SessionError> {
        let assets = self
            .catalog
            .with_transaction(list_assets)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        Ok(assets
            .into_iter()
            .map(|asset| AssetSummaryWire {
                id: asset.id.to_string(),
                path: asset.original.path.to_string_lossy().into_owned(),
                codec: asset
                    .original
                    .codec
                    .clone()
                    .unwrap_or_else(|| "unknown".to_owned()),
                duration_millis: asset.original.duration_millis.unwrap_or(0),
                imported_at_millis: asset.original.imported_at_millis,
                max_level: asset.max_level as u8,
            })
            .collect())
    }

    /// Total registered asset count.
    #[must_use]
    pub fn asset_count(&self) -> u64 {
        self.catalog.stats().map_or(0, |stats| stats.asset_count)
    }

    /// The catalog file path.
    #[must_use]
    pub fn catalog_path(&self) -> &Path {
        &self.catalog_path
    }
}
