//! The long-lived Library session: one catalog attachment for the desktop
//! process lifetime.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use echo_catalog::{AssetLookup, Catalog, find_by_id, list_assets, open_catalog, query_analysis};
use echo_core::load_or_build_waveform;
use echo_domain::AssetId;

use crate::ffi::{
    AnalysisStatusWire, AssetSummaryWire, JobStatsWire, KeywordFacetWire, ScanRootWire,
    SearchHitWire, SmartAlbumWire, TranscriptSegmentWire, TranscriptWire, WaveformArtifactWire,
    WaveformLevelWire,
};

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

fn metadata_entry(entries: &[echo_catalog::SourceMetadataEntry], keys: &[&str]) -> String {
    entries
        .iter()
        .find(|entry| keys.iter().any(|key| entry.key.eq_ignore_ascii_case(key)))
        .map_or_else(String::new, |entry| entry.value.clone())
}

fn metadata_entry_containing(
    entries: &[echo_catalog::SourceMetadataEntry],
    needle: &str,
) -> String {
    entries
        .iter()
        .find(|entry| entry.key.to_ascii_lowercase().contains(needle))
        .map_or_else(String::new, |entry| entry.value.clone())
}

const fn job_state_text(state: echo_catalog::JobState) -> &'static str {
    match state {
        echo_catalog::JobState::Pending => "pending",
        echo_catalog::JobState::Running => "running",
        echo_catalog::JobState::Done => "done",
        echo_catalog::JobState::Failed => "failed",
        echo_catalog::JobState::Cancelled => "cancelled",
    }
}

struct AdjustmentWireFields {
    revision: i64,
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: u8,
    fade_out_curve: u8,
    gain_centibels: i16,
}

fn adjustment_wire_fields(
    adjustment: Option<echo_catalog::AssetAdjustmentRevision>,
    source_duration_millis: Option<u64>,
) -> AdjustmentWireFields {
    adjustment.map_or_else(
        || AdjustmentWireFields {
            revision: 0,
            trim_start_millis: 0,
            trim_end_millis: source_duration_millis.unwrap_or(0),
            fade_in_millis: 0,
            fade_out_millis: 0,
            fade_in_curve: 0,
            fade_out_curve: 0,
            gain_centibels: 0,
        },
        |revision| AdjustmentWireFields {
            revision: revision.revision_id,
            trim_start_millis: revision.graph.trim_start_millis(),
            trim_end_millis: revision.graph.trim_end_millis(),
            fade_in_millis: revision.graph.fade_in_millis(),
            fade_out_millis: revision.graph.fade_out_millis(),
            fade_in_curve: u8::try_from(revision.graph.fade_in_curve().catalog_value())
                .expect("fade curve catalog values fit u8"),
            fade_out_curve: u8::try_from(revision.graph.fade_out_curve().catalog_value())
                .expect("fade curve catalog values fit u8"),
            gain_centibels: revision.graph.gain_centibels(),
        },
    )
}

fn asset_summary_wire(asset: echo_catalog::AudioSpaceAsset) -> AssetSummaryWire {
    let adjustment = adjustment_wire_fields(asset.adjustment, asset.duration_millis);
    let (sound_caption, summary) = asset
        .contextual
        .as_ref()
        .and_then(|value| {
            serde_json::from_value::<echo_core::ContextualPayload>(value.clone()).ok()
        })
        .map_or((String::new(), String::new()), |payload| {
            (
                if payload.is_current() {
                    payload.sound_caption
                } else {
                    String::new()
                },
                payload.summary,
            )
        });
    let text_preview = asset
        .transcript
        .as_ref()
        .and_then(|value| {
            serde_json::from_value::<echo_core::TranscriptPayload>(value.clone()).ok()
        })
        .map_or_else(String::new, |payload| payload.text);
    let (
        container_format,
        sample_rate,
        channel_count,
        source_title,
        source_location,
        source_created_at,
    ) = asset.source_metadata.as_ref().map_or_else(
        || {
            (
                String::new(),
                0,
                0,
                String::new(),
                String::new(),
                String::new(),
            )
        },
        |metadata| {
            (
                metadata.container_format.clone(),
                metadata.sample_rate,
                metadata.channel_count,
                metadata_entry(&metadata.entries, &["title", "stream.title"]),
                metadata_entry_containing(&metadata.entries, "location"),
                metadata_entry(
                    &metadata.entries,
                    &["creation_time", "stream.creation_time"],
                ),
            )
        },
    );
    AssetSummaryWire {
        id: asset.id,
        path: asset.path.to_string_lossy().into_owned(),
        codec: asset.codec.unwrap_or_else(|| "unknown".to_owned()),
        duration_millis: asset.duration_millis.unwrap_or(0),
        recorded_at_millis: asset.recorded_at_millis.unwrap_or(0),
        imported_at_millis: asset.imported_at_millis,
        max_level: asset.max_level,
        path_status: asset.path_status,
        sound_caption,
        summary,
        event_type: asset.contextual_event_type.unwrap_or_default(),
        mood: asset.contextual_mood.unwrap_or_default(),
        keywords: asset.contextual_keywords,
        text_preview,
        liked: asset.liked,
        rating: asset.rating,
        adjustment_revision: adjustment.revision,
        trim_start_millis: adjustment.trim_start_millis,
        trim_end_millis: adjustment.trim_end_millis,
        fade_in_millis: adjustment.fade_in_millis,
        fade_out_millis: adjustment.fade_out_millis,
        fade_in_curve: adjustment.fade_in_curve,
        fade_out_curve: adjustment.fade_out_curve,
        gain_centibels: adjustment.gain_centibels,
        container_format,
        sample_rate,
        channel_count,
        source_title,
        source_location,
        source_created_at,
    }
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
        let _ = self
            .catalog
            .with_transaction(list_assets)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let projection = self
            .catalog
            .with_transaction(echo_catalog::list_audio_space)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        Ok(projection.into_iter().map(asset_summary_wire).collect())
    }

    /// Lists contextual keyword facets using the Catalog's latest-evidence
    /// projection rather than aggregating model strings in QML.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the aggregate query fails.
    pub fn keyword_facets(&self) -> Result<Vec<KeywordFacetWire>, SessionError> {
        let facets = self
            .catalog
            .with_transaction(echo_catalog::list_contextual_keyword_facets)
            .map_err(SessionError::from)?;
        Ok(facets
            .into_iter()
            .map(|facet| KeywordFacetWire {
                key: facet.key,
                label: facet.label,
                count: facet.count,
            })
            .collect())
    }

    /// Lists smart album candidates projected by the Catalog. Membership is
    /// explicit so QML never needs to reinterpret evidence strings.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the candidate projection fails.
    pub fn smart_albums(&self) -> Result<Vec<SmartAlbumWire>, SessionError> {
        let candidates = self
            .catalog
            .with_transaction(echo_catalog::list_smart_album_candidates)
            .map_err(SessionError::from)?;
        Ok(candidates
            .into_iter()
            .map(|candidate| SmartAlbumWire {
                key: candidate.key,
                label: candidate.label,
                facet: candidate.facet.as_str().to_owned(),
                evidence: candidate.evidence.as_str().to_owned(),
                count: u64::try_from(candidate.member_asset_ids.len()).unwrap_or(u64::MAX),
                member_asset_ids: candidate
                    .member_asset_ids
                    .into_iter()
                    .map(|asset_id| asset_id.to_string())
                    .collect(),
            })
            .collect())
    }

    /// Stores user-owned Like and rating state for one sound.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset identity or write is invalid.
    pub fn set_asset_affinity(
        &self,
        asset_id: &str,
        liked: bool,
        rating: u8,
    ) -> Result<(), SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::set_asset_affinity(
                    transaction,
                    asset_id,
                    echo_catalog::AssetAffinity { liked, rating },
                    now_millis(),
                )
            })
            .map_err(SessionError::from)
    }

    /// Appends one validated, non-destructive adjustment graph.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset is unknown, lacks a duration,
    /// or the authored range, fades, or gain violate the product contract.
    pub fn set_asset_adjustment(
        &self,
        asset_id: &str,
        adjustment: &crate::ffi::AssetAdjustmentWire,
    ) -> Result<(), SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let duration = self.catalog.with_transaction(|transaction| {
            match find_by_id(transaction, asset_id) {
                Ok(AssetLookup::Found(asset)) => {
                    asset.original.duration_millis.ok_or_else(|| SessionError {
                        message: "asset duration is not available".to_owned(),
                    })
                }
                Ok(AssetLookup::NotFound) => Err(SessionError {
                    message: format!("asset {asset_id} not found"),
                }),
                Err(error) => Err(SessionError {
                    message: error.to_string(),
                }),
            }
        })?;
        let graph = echo_domain::AdjustmentGraph::new(
            duration,
            adjustment.trim_start_millis,
            adjustment.trim_end_millis,
            adjustment.fade_in_millis,
            adjustment.fade_out_millis,
            echo_domain::FadeCurves::new(
                echo_domain::FadeCurve::from_catalog_value(i64::from(adjustment.fade_in_curve))
                    .map_err(|error| SessionError {
                        message: error.to_string(),
                    })?,
                echo_domain::FadeCurve::from_catalog_value(i64::from(adjustment.fade_out_curve))
                    .map_err(|error| SessionError {
                        message: error.to_string(),
                    })?,
            ),
            adjustment.gain_centibels,
        )
        .map_err(|error| SessionError {
            message: error.to_string(),
        })?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::record_adjustment_graph(transaction, asset_id, graph, now_millis())
            })
            .map(|_| ())
            .map_err(SessionError::from)
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
        let payload = load_or_build_waveform(&self.catalog, id, &source, &self.cache_root, 8)
            .map_err(|error| SessionError {
                message: format!("cannot build waveform for {}: {error}", source.display()),
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
        let alignment = records
            .iter()
            .find(|record| record.kind == echo_domain::AnalysisKind::Alignment)
            .and_then(|record| {
                serde_json::from_value::<echo_core::AlignmentPayload>(record.value.clone()).ok()
            });
        for record in records {
            if record.kind != echo_domain::AnalysisKind::Transcript {
                continue;
            }
            let Ok(payload) = serde_json::from_value::<echo_core::TranscriptPayload>(record.value)
            else {
                continue;
            };
            let segments = aligned_segments(&payload, alignment.as_ref());
            wires.push(TranscriptWire {
                model: record.model.name,
                model_version: record.model.version,
                language: payload.language.unwrap_or_default(),
                text: payload.text,
                segments: segments
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
    pub fn start_workers(&self, runtime_endpoint: &str) -> Result<(), SessionError> {
        let mut workers = self.workers.lock().expect("worker mutex poisoned");
        if workers.is_some() {
            return Ok(());
        }
        // Missing or unsafe credentials must not prevent structural Library
        // jobs from running. Inference then fails through its sanitized
        // authentication path while scanning and waveform work continues.
        let bearer_token = echo_core::load_infer_runtime_credential().map_or_else(
            |_| String::new(),
            echo_core::InferRuntimeCredential::into_bearer_token,
        );
        let config = echo_core::WorkerConfig {
            cache_root: self.cache_root.clone(),
            infer_runtime: echo_core::InferRuntimeConfig {
                base_url: runtime_endpoint.to_owned(),
                bearer_token,
            },
        };
        let pool = echo_core::WorkerPool::start(&self.catalog, &config, 2).map_err(|error| {
            SessionError {
                message: format!("cannot start workers: {error}"),
            }
        })?;
        *workers = Some(pool);
        Ok(())
    }

    /// Returns the product stage and sanitized Runtime linkage for one asset.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset identity or catalog projection
    /// is invalid.
    pub fn analysis_status(&self, asset_id: &str) -> Result<AnalysisStatusWire, SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        self.catalog.with_transaction(|transaction| {
            let records = query_analysis(transaction, asset_id).map_err(|error| SessionError {
                message: error.to_string(),
            })?;
            let has_transcript = records
                .iter()
                .any(|record| record.kind == echo_domain::AnalysisKind::Transcript);
            let latest_transcript_is_empty = records
                .iter()
                .find(|record| record.kind == echo_domain::AnalysisKind::Transcript)
                .and_then(|record| {
                    serde_json::from_value::<echo_core::TranscriptPayload>(record.value.clone())
                        .ok()
                })
                .is_some_and(|payload| payload.text.trim().is_empty());
            let has_alignment = records
                .iter()
                .any(|record| record.kind == echo_domain::AnalysisKind::Alignment);
            let has_current_contextual = records
                .iter()
                .find(|record| record.kind == echo_domain::AnalysisKind::Contextual)
                .and_then(|record| {
                    serde_json::from_value::<echo_core::ContextualPayload>(record.value.clone())
                        .ok()
                })
                .is_some_and(|payload| payload.is_current());
            let contextual_job_id = echo_core::contextual_job_id(asset_id);
            let (stage, job_id) = if latest_transcript_is_empty {
                ("complete", format!("align-{asset_id}"))
            } else if has_current_contextual {
                ("complete", contextual_job_id.clone())
            } else if has_alignment {
                ("contextual", contextual_job_id)
            } else if has_transcript {
                ("alignment", format!("align-{asset_id}"))
            } else {
                ("text", format!("transcribe-{asset_id}"))
            };
            let job = echo_catalog::job_by_id(transaction, &job_id)?;
            let run = echo_catalog::inference_run(transaction, &job_id)?;
            Ok(AnalysisStatusWire {
                stage: stage.to_owned(),
                state: job.as_ref().map_or_else(
                    || {
                        if has_current_contextual || latest_transcript_is_empty {
                            "done"
                        } else {
                            "missing"
                        }
                        .to_owned()
                    },
                    |job| job_state_text(job.state).to_owned(),
                ),
                error_code: run
                    .as_ref()
                    .and_then(|run| run.error_code.clone())
                    .unwrap_or_default(),
                runtime_job_id: run
                    .as_ref()
                    .and_then(|run| run.runtime_job_id.clone())
                    .unwrap_or_default(),
                contract_version: run
                    .as_ref()
                    .map_or_else(String::new, |run| run.contract_version.clone()),
            })
        })
    }

    /// Requeues the failed product analysis stage for one asset.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when no failed stage can be retried.
    pub fn retry_analysis(&self, asset_id: &str) -> Result<(), SessionError> {
        let status = self.analysis_status(asset_id)?;
        let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let job_id = match status.stage.as_str() {
            "contextual" => echo_core::contextual_job_id(id),
            "alignment" => format!("align-{id}"),
            _ => format!("transcribe-{id}"),
        };
        let retried = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::retry_job(transaction, &job_id, now_millis())
            })
            .map_err(SessionError::from)?;
        if retried {
            Ok(())
        } else {
            Err(SessionError {
                message: "analysis stage is not failed or cancelled".to_owned(),
            })
        }
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

    /// Full-text search over indexed transcripts. Each hit carries the best
    /// matching segment's start time so the UI can jump straight to it.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the search fails.
    pub fn search(&self, query: &str, limit: u64) -> Result<Vec<SearchHitWire>, SessionError> {
        let hits = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::search_transcripts(transaction, query, limit)
            })
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let mut wires = Vec::new();
        for hit in hits {
            let id = AssetId::from_str(&hit.asset_id).map_err(|error| SessionError {
                message: format!("bad asset id {}: {error}", hit.asset_id),
            })?;
            let asset =
                self.catalog
                    .with_transaction(|transaction| match find_by_id(transaction, id) {
                        Ok(AssetLookup::Found(asset)) => Ok(asset),
                        Ok(AssetLookup::NotFound) => Err(SessionError {
                            message: format!("asset {} not found", hit.asset_id),
                        }),
                        Err(error) => Err(SessionError {
                            message: error.to_string(),
                        }),
                    })?;
            let start_millis = self.best_segment_start_millis(id, query)?;
            wires.push(SearchHitWire {
                asset_id: hit.asset_id,
                path: asset.original.path.to_string_lossy().into_owned(),
                codec: asset
                    .original
                    .codec
                    .clone()
                    .unwrap_or_else(|| "unknown".to_owned()),
                snippet: hit.snippet,
                start_millis,
            });
        }
        Ok(wires)
    }

    /// Finds the transcript segment whose text best overlaps the query and
    /// returns its start time in milliseconds.
    fn best_segment_start_millis(
        &self,
        asset_id: AssetId,
        query: &str,
    ) -> Result<u64, SessionError> {
        let records = self
            .catalog
            .with_transaction(|transaction| query_analysis(transaction, asset_id))
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let transcript = records
            .iter()
            .find(|record| record.kind == echo_domain::AnalysisKind::Transcript)
            .and_then(|record| {
                serde_json::from_value::<echo_core::TranscriptPayload>(record.value.clone()).ok()
            });
        let Some(transcript) = transcript else {
            return Ok(0);
        };
        let query_chars: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).collect();
        let mut best_start = None;
        let mut best_overlap = 0usize;
        for segment in &transcript.segments {
            let overlap = query_chars
                .iter()
                .filter(|c| segment.text.contains(**c))
                .count();
            if overlap > best_overlap {
                best_overlap = overlap;
                best_start = Some(segment.start);
            }
        }
        // Segment starts are seconds bounded by recording length; the
        // float-to-int conversion cannot lose meaningful precision here.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let start_millis = (best_start.unwrap_or(0.0).max(0.0) * 1000.0) as u64;
        Ok(start_millis)
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

pub(crate) fn aligned_segments(
    transcript: &echo_core::TranscriptPayload,
    alignment: Option<&echo_core::AlignmentPayload>,
) -> Vec<echo_core::TranscriptSegment> {
    let Some(alignment) = alignment.filter(|alignment| {
        normalized_text(&alignment.text) == normalized_text(&transcript.text)
            && !alignment.items.is_empty()
    }) else {
        return transcript.segments.clone();
    };
    let mut cursor = 0;
    transcript
        .segments
        .iter()
        .map(|segment| {
            let target = normalized_text(&segment.text);
            let start_cursor = cursor;
            let mut collected = String::new();
            while cursor < alignment.items.len()
                && collected.chars().count() < target.chars().count()
            {
                collected.push_str(&normalized_text(&alignment.items[cursor].text));
                cursor += 1;
            }
            if !target.is_empty() && collected == target && cursor > start_cursor {
                let mut refined = segment.clone();
                refined.start = alignment.items[start_cursor].start;
                refined.end = alignment.items[cursor - 1].end;
                refined
            } else {
                cursor = start_cursor;
                segment.clone()
            }
        })
        .collect()
}

fn normalized_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
