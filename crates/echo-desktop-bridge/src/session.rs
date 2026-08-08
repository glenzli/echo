//! The long-lived Library session: one catalog attachment for the desktop
//! process lifetime.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use echo_cache::{open_blob_store, read_verified};
use echo_catalog::{AssetLookup, Catalog, find_by_id, list_assets, open_catalog, query_analysis};
use echo_core::{WaveformArtifactPayload, build_and_cache_waveform};
use echo_domain::AssetId;

use crate::ffi::{
    AssetSummaryWire, JobStatsWire, ScanRootWire, TranscriptSegmentWire, TranscriptWire,
    WaveformArtifactWire, WaveformLevelWire,
};

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

/// One catalog attachment. Sessions are created on the Qt main thread and
/// reused; the catalog serializes its own writes.
#[derive(Debug)]
pub struct LibrarySession {
    catalog: std::sync::Arc<Catalog>,
    catalog_path: PathBuf,
    cache_root: PathBuf,
    workers: std::sync::Mutex<Option<echo_core::WorkerPool>>,
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

/// Transcribes an asset with the configured MLX worker. Stateless so it can
/// run on a background thread.
///
/// # Errors
///
/// Returns [`SessionError`] when the asset is unknown, the ASR model is
/// missing, or the worker fails.
pub fn transcribe_asset(
    catalog_path: &str,
    asset_id: &str,
    model_root: &str,
    python: &str,
    worker_script: &str,
) -> Result<u32, SessionError> {
    let catalog =
        open_catalog(std::path::Path::new(catalog_path)).map_err(|error| SessionError {
            message: error.to_string(),
        })?;
    let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
        message: format!("invalid asset id {asset_id}: {error}"),
    })?;
    let source = catalog.with_transaction(|transaction| match find_by_id(transaction, id) {
        Ok(AssetLookup::Found(asset)) => Ok(asset.original.path),
        Ok(AssetLookup::NotFound) => Err(SessionError {
            message: format!("asset {asset_id} not found"),
        }),
        Err(error) => Err(SessionError {
            message: error.to_string(),
        }),
    })?;
    let asr_spec = echo_ai::MODEL_CATALOG
        .iter()
        .find(|spec| spec.id == "qwen3-asr-1.7b-mlx")
        .expect("catalog declares qwen3-asr");
    let snapshot = match echo_ai::resolve_model(std::path::Path::new(model_root), asr_spec)
        .map_err(|error| SessionError {
            message: format!("cannot scan model root: {error}"),
        })? {
        echo_ai::ModelStatus::Present { snapshot } => snapshot,
        echo_ai::ModelStatus::Missing { download_command } => {
            return Err(SessionError {
                message: format!("ASR model missing; run: {download_command}"),
            });
        }
    };
    let worker = echo_core::TranscribeWorker {
        python: std::path::PathBuf::from(python),
        script: std::path::PathBuf::from(worker_script),
    };
    let payload =
        echo_core::run_transcribe(&source, &snapshot, &worker).map_err(|error| SessionError {
            message: format!("transcription failed: {error}"),
        })?;
    let version = snapshot
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    echo_core::record_transcript(&catalog, id, &payload, &version).map_err(|error| {
        SessionError {
            message: error.to_string(),
        }
    })?;
    Ok(u32::try_from(payload.segments.len()).expect("segment count fits u32"))
}

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
        catalog: std::sync::Arc::new(catalog),
        catalog_path,
        cache_root: PathBuf::from(cache_root),
        workers: std::sync::Mutex::new(None),
    })
}

impl Drop for LibrarySession {
    fn drop(&mut self) {
        if let Some(workers) = self.workers.lock().expect("worker mutex poisoned").take() {
            workers.stop();
        }
    }
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
                path_status: match asset.original.path_status {
                    echo_domain::AssetPathStatus::Present => "present",
                    echo_domain::AssetPathStatus::Missing => "missing",
                }
                .to_owned(),
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

    /// Returns every transcript evidence record for an asset, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the catalog read fails.
    pub fn transcripts(&self, asset_id: &str) -> Result<Vec<TranscriptWire>, SessionError> {
        let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let records = self
            .catalog
            .with_transaction(|transaction| query_analysis(transaction, id))
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let mut wires = Vec::new();
        for record in records {
            if record.kind != echo_domain::AnalysisKind::Transcript {
                continue;
            }
            let Ok(payload) = serde_json::from_value::<echo_core::TranscriptPayload>(record.value)
            else {
                continue;
            };
            wires.push(TranscriptWire {
                model: record.model.name,
                model_version: record.model.version,
                language: payload.language.unwrap_or_default(),
                text: payload.text,
                segments: payload
                    .segments
                    .into_iter()
                    .map(|segment| TranscriptSegmentWire {
                        text: segment.text,
                        start: segment.start,
                        end: segment.end,
                    })
                    .collect(),
            });
        }
        Ok(wires)
    }

    /// Starts the background worker pool (idempotent).
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the pool cannot start.
    pub fn start_workers(
        &self,
        model_root: &str,
        python: &str,
        worker_script: &str,
        ollama_endpoint: &str,
        ollama_model: &str,
    ) -> Result<(), SessionError> {
        let mut workers = self.workers.lock().expect("worker mutex poisoned");
        if workers.is_some() {
            return Ok(());
        }
        let config = echo_core::WorkerConfig {
            cache_root: self.cache_root.clone(),
            model_root: PathBuf::from(model_root),
            python: PathBuf::from(python),
            worker_script: PathBuf::from(worker_script),
            ollama_endpoint: ollama_endpoint.to_owned(),
            ollama_model: ollama_model.to_owned(),
        };
        let pool = echo_core::WorkerPool::start(&self.catalog, &config, 2).map_err(|error| {
            SessionError {
                message: format!("cannot start workers: {error}"),
            }
        })?;
        *workers = Some(pool);
        Ok(())
    }

    /// Queues scans for every enabled root (incremental detection).
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when queueing fails.
    pub fn queue_scans(&self) -> Result<u64, SessionError> {
        echo_core::queue_scans_for_enabled_roots(&self.catalog, now_millis()).map_err(|error| {
            SessionError {
                message: error.to_string(),
            }
        })
    }

    /// Reads aggregate job statistics.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the read fails.
    pub fn job_stats(&self) -> Result<JobStatsWire, SessionError> {
        let stats = self
            .catalog
            .with_transaction(echo_catalog::job_stats)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        Ok(JobStatsWire {
            pending: stats.pending,
            running: stats.running,
            done: stats.done,
            failed: stats.failed,
        })
    }

    /// Lists configured scan roots.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the read fails.
    pub fn list_roots(&self) -> Result<Vec<ScanRootWire>, SessionError> {
        let roots = self
            .catalog
            .with_transaction(echo_catalog::list_scan_roots)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        Ok(roots
            .into_iter()
            .map(|root| ScanRootWire {
                id: root.id,
                root: root.root.to_string_lossy().into_owned(),
                enabled: root.enabled,
            })
            .collect())
    }

    /// Adds a scan root and queues its scan.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the write fails.
    pub fn add_root(&self, root: &str) -> Result<(), SessionError> {
        echo_core::add_root_and_scan(&self.catalog, std::path::Path::new(root), now_millis())
            .map_err(|error| SessionError {
                message: error.to_string(),
            })
    }

    /// Removes a scan root by id.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the write fails.
    pub fn remove_root(&self, id: i64) -> Result<(), SessionError> {
        self.catalog
            .with_transaction(|transaction| echo_catalog::remove_scan_root(transaction, id))
            .map_err(|error| SessionError {
                message: error.to_string(),
            })
    }

    /// Exposes the catalog for contract tests.
    #[cfg(test)]
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
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
