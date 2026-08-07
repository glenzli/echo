//! The long-lived Library session: one catalog attachment for the desktop
//! process lifetime.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use echo_cache::{open_blob_store, read_verified};
use echo_catalog::{AssetLookup, Catalog, find_by_id, list_assets, open_catalog};
use echo_core::{WaveformArtifactPayload, build_and_cache_waveform};
use echo_domain::AssetId;

use crate::ffi::{AssetSummaryWire, WaveformArtifactWire, WaveformLevelWire};

/// One catalog attachment. Sessions are created on the Qt main thread and
/// reused; the catalog serializes its own writes.
#[derive(Debug)]
pub struct LibrarySession {
    catalog: Catalog,
    catalog_path: PathBuf,
    cache_root: PathBuf,
}

/// Error vocabulary for session operations.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct SessionError {
    pub message: String,
}

impl From<echo_catalog::CatalogError> for SessionError {
    fn from(error: echo_catalog::CatalogError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

/// Upper bound for a cached waveform artifact read (a few hours of base-level
/// min/max pairs stay far below this).
const MAX_WAVEFORM_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;

/// Opens (creating if needed) the catalog at `path` with the cache root at
/// `cache_root`.
///
/// # Errors
///
/// Returns [`SessionError`] when the catalog cannot be opened.
pub fn open_session(path: &str, cache_root: &str) -> Result<LibrarySession, SessionError> {
    let catalog_path = PathBuf::from(path);
    let catalog = open_catalog(&catalog_path).map_err(|error| SessionError {
        message: error.to_string(),
    })?;
    Ok(LibrarySession {
        catalog,
        catalog_path,
        cache_root: PathBuf::from(cache_root),
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

    /// Returns the waveform artifact for an asset, building and caching it
    /// when absent.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset is unknown or the artifact
    /// cannot be built, read, or decoded.
    pub fn waveform_artifact(&self, asset_id: &str) -> Result<WaveformArtifactWire, SessionError> {
        let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let source =
            self.catalog
                .with_transaction(|transaction| match find_by_id(transaction, id) {
                    Ok(AssetLookup::Found(asset)) => Ok(asset.original.path),
                    Ok(AssetLookup::NotFound) => Err(SessionError {
                        message: format!("asset {asset_id} not found"),
                    }),
                    Err(error) => Err(SessionError {
                        message: error.to_string(),
                    }),
                })?;
        let artifact = build_and_cache_waveform(&source, &self.cache_root, 8).map_err(|error| {
            SessionError {
                message: format!("cannot build waveform for {}: {error}", source.display()),
            }
        })?;
        let store = open_blob_store(&self.cache_root).map_err(|error| SessionError {
            message: error.to_string(),
        })?;
        let bytes = read_verified(&store, artifact.content_hash, MAX_WAVEFORM_ARTIFACT_BYTES)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let payload: WaveformArtifactPayload =
            serde_json::from_slice(&bytes).map_err(|error| SessionError {
                message: format!("cannot decode cached waveform artifact: {error}"),
            })?;
        Ok(WaveformArtifactWire {
            canonical_sample_rate: payload.canonical_sample_rate,
            levels: payload
                .levels
                .into_iter()
                .map(|level| WaveformLevelWire {
                    samples_per_bucket: level.samples_per_bucket,
                    mins: level.mins,
                    maxs: level.maxs,
                })
                .collect(),
        })
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

    /// The cache root path.
    #[must_use]
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }
}
