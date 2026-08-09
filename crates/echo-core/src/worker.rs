//! The background job worker pool.
//!
//! Workers claim persisted jobs one at a time, dispatch by kind, and commit
//! state transitions with the catalog. Recovery resets interrupted jobs on
//! startup, so a crash never loses work.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, UNIX_EPOCH},
};

use echo_catalog::{
    AssetLookup, Catalog, ClaimedJob, FileJobPayload, InferenceRunState, JobKind,
    ScanRootJobPayload, UpsertInferenceRun, claim_next_job, complete_job, enqueue_job, fail_job,
    find_by_content_hash, find_by_id, recover_interrupted_jobs, requeue_recoverable_inference_runs,
    upsert_inference_run, upsert_journal,
};

use crate::{
    analysis_queue,
    error::{CoreError, CoreErrorKind},
    metadata_queue, scanner,
};

/// Worker configuration shared by every job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    pub cache_root: PathBuf,
    pub infer_runtime: crate::InferRuntimeConfig,
}

/// The worker pool handle.
#[derive(Debug)]
pub struct WorkerPool {
    stop: Arc<AtomicBool>,
    handles: Vec<thread::JoinHandle<()>>,
}

impl WorkerPool {
    /// Spawns `worker_count` workers over `catalog`, after resetting
    /// interrupted jobs.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] when the recovery pass fails.
    pub fn start(
        catalog: &Arc<Catalog>,
        config: &WorkerConfig,
        worker_count: usize,
    ) -> Result<Self, CoreError> {
        let now = crate::util::now_millis();
        catalog
            .with_transaction(|transaction| recover_interrupted_jobs(transaction, now))
            .map_err(CoreError::from)?;
        catalog
            .with_transaction(|transaction| {
                requeue_recoverable_inference_runs(
                    transaction,
                    !config.infer_runtime.bearer_token.trim().is_empty(),
                    now,
                )
            })
            .map_err(CoreError::from)?;
        metadata_queue::enqueue_missing_source_metadata(catalog, now)?;
        analysis_queue::enqueue_missing_transcriptions(catalog, now)?;
        analysis_queue::settle_empty_transcript_alignments(catalog, now)?;
        analysis_queue::enqueue_missing_alignments(catalog, now)?;
        analysis_queue::enqueue_missing_contextual(catalog, now)?;

        let stop = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();
        for _ in 0..worker_count {
            let catalog = Arc::clone(catalog);
            let config = config.clone();
            let stop = stop.clone();
            handles.push(thread::spawn(move || {
                worker_loop(&catalog, &config, &stop);
            }));
        }
        Ok(Self { stop, handles })
    }

    /// Stops the pool, waiting for in-flight jobs.
    pub fn stop(self) {
        self.stop.store(true, Ordering::Release);
        for handle in self.handles {
            let _ = handle.join();
        }
    }
}

fn worker_loop(catalog: &Catalog, config: &WorkerConfig, stop: &AtomicBool) {
    while !stop.load(Ordering::Acquire) {
        let claimed = catalog
            .with_transaction(|transaction| claim_next_job(transaction, crate::util::now_millis()))
            .ok()
            .flatten();
        let Some(job) = claimed else {
            thread::sleep(Duration::from_millis(200));
            continue;
        };
        let result = dispatch(catalog, config, &job);
        let now = crate::util::now_millis();
        match result {
            Ok(()) => {
                let _ =
                    catalog.with_transaction(|transaction| complete_job(transaction, &job.id, now));
            }
            Err(error) => {
                let _ = catalog.with_transaction(|transaction| {
                    fail_job(transaction, &job.id, &error.to_string(), now)
                });
            }
        }
    }
}

fn dispatch(catalog: &Catalog, config: &WorkerConfig, job: &ClaimedJob) -> Result<(), CoreError> {
    match job.kind {
        JobKind::ScanRoot => {
            let payload = ScanRootJobPayload::decode(&job.payload)?;
            scanner::scan_root(catalog, &payload.root, crate::util::now_millis())?;
            metadata_queue::enqueue_missing_source_metadata(catalog, crate::util::now_millis())?;
            analysis_queue::enqueue_missing_transcriptions(catalog, crate::util::now_millis())?;
            analysis_queue::enqueue_missing_alignments(catalog, crate::util::now_millis())?;
            analysis_queue::enqueue_missing_contextual(catalog, crate::util::now_millis())?;
            Ok(())
        }
        JobKind::ImportFile => {
            let payload = FileJobPayload::decode(&job.payload)?;
            import_file(catalog, config, &payload.path)?;
            Ok(())
        }
        JobKind::ExtractMetadata => {
            let asset_id = asset_id_of(&job.payload)?;
            let source = source_path_of(catalog, &asset_id)?;
            let probe = echo_bridge::probe(&source).map_err(|error| {
                CoreError::new(CoreErrorKind::AudioEngineRejected, error.message)
            })?;
            catalog.with_transaction(|transaction| {
                echo_catalog::record_source_metadata(
                    transaction,
                    asset_id,
                    &echo_catalog::SourceMetadata {
                        container_format: probe.container_format,
                        sample_rate: probe.sample_rate,
                        channel_count: probe.channel_count,
                        entries: probe
                            .metadata
                            .into_iter()
                            .map(|entry| echo_catalog::SourceMetadataEntry {
                                key: entry.key,
                                value: entry.value,
                            })
                            .collect(),
                    },
                    (probe.recorded_at_millis > 0).then_some(probe.recorded_at_millis),
                )
                .map_err(CoreError::from)
            })
        }
        JobKind::AnalyzeWaveform => {
            let asset_id = asset_id_of(&job.payload)?;
            let source = source_path_of(catalog, &asset_id)?;
            crate::waveform_artifact::load_or_build_waveform(
                catalog,
                asset_id,
                &source,
                &config.cache_root,
                8,
            )
            .map(|_| ())
        }
        JobKind::Transcribe | JobKind::Align | JobKind::Contextual => {
            dispatch_analysis(catalog, config, job)
        }
    }
}

fn dispatch_analysis(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    match job.kind {
        JobKind::Transcribe => dispatch_transcription(catalog, config, job),
        JobKind::Align => dispatch_alignment(catalog, config, job),
        JobKind::Contextual => dispatch_contextual(catalog, config, job),
        JobKind::ScanRoot
        | JobKind::ImportFile
        | JobKind::ExtractMetadata
        | JobKind::AnalyzeWaveform => unreachable!("structural jobs use dispatch"),
    }
}

fn dispatch_transcription(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    let asset_id = asset_id_of(&job.payload)?;
    let source = source_path_of(catalog, &asset_id)?;
    record_inference_submitting(catalog, job, asset_id, crate::TRANSCRIPTION_INTENT)?;
    let client = crate::InferRuntimeClient::new(config.infer_runtime.clone());
    let payload = match client.transcribe(&source, &crate::TranscriptionIntent::default()) {
        Ok(payload) => payload,
        Err(error) => {
            record_inference_failure(catalog, job, asset_id, crate::TRANSCRIPTION_INTENT, &error)?;
            return Err(error.into());
        }
    };
    let provenance = payload.runtime.as_ref().ok_or_else(|| {
        CoreError::new(
            CoreErrorKind::InferenceRejected,
            "Runtime transcript lacks accepted Job provenance",
        )
    })?;
    record_inference_success(catalog, job, asset_id, provenance)?;
    crate::record_runtime_transcript(catalog, asset_id, &payload)?;
    if !payload.text.trim().is_empty() {
        catalog.with_transaction(|transaction| {
            analysis_queue::enqueue_alignment(transaction, asset_id, crate::util::now_millis())
                .map_err(CoreError::from)
        })?;
    }
    Ok(())
}

fn dispatch_alignment(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    let asset_id = asset_id_of(&job.payload)?;
    let source = source_path_of(catalog, &asset_id)?;
    let transcript = newest_transcript(catalog, asset_id)?;
    record_inference_submitting(catalog, job, asset_id, crate::ALIGNMENT_INTENT)?;
    let client = crate::InferRuntimeClient::new(config.infer_runtime.clone());
    let intent = crate::AlignmentIntent {
        language: transcript.language.clone(),
        ..crate::AlignmentIntent::default()
    };
    let payload = match client.align(&source, &transcript.text, &intent) {
        Ok(payload) => payload,
        Err(error) => {
            record_inference_failure(catalog, job, asset_id, crate::ALIGNMENT_INTENT, &error)?;
            return Err(error.into());
        }
    };
    record_inference_success(catalog, job, asset_id, &payload.runtime)?;
    crate::record_alignment(catalog, asset_id, &payload)?;
    catalog.with_transaction(|transaction| {
        analysis_queue::enqueue_contextual(transaction, asset_id, crate::util::now_millis())
            .map_err(CoreError::from)
    })
}

fn dispatch_contextual(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    let asset_id = asset_id_of(&job.payload)?;
    let transcript = newest_transcript(catalog, asset_id)?;
    record_inference_submitting(catalog, job, asset_id, crate::CONTEXTUAL_INTENT)?;
    let client = crate::InferRuntimeClient::new(config.infer_runtime.clone());
    let response = match client.contextualize(&transcript.text, &crate::ContextualIntent::default())
    {
        Ok(response) => response,
        Err(error) => {
            record_inference_failure(catalog, job, asset_id, crate::CONTEXTUAL_INTENT, &error)?;
            return Err(error.into());
        }
    };
    let payload = match crate::contextual::decode_contextual_output(&response.output_text) {
        Ok(payload) => payload,
        Err(error) => {
            record_inference_ingestion_failure(
                catalog,
                job,
                asset_id,
                &response.runtime,
                error.code,
            )?;
            return Err(CoreError::new(
                CoreErrorKind::InferenceRejected,
                error.to_string(),
            ));
        }
    };
    record_inference_success(catalog, job, asset_id, &response.runtime)?;
    crate::record_contextual(catalog, asset_id, &payload, &response.runtime)
}

/// Imports one discovered file: hash, relink or register, probe, journal.
fn import_file(catalog: &Catalog, _config: &WorkerConfig, path: &Path) -> Result<(), CoreError> {
    let content_hash = crate::import::hash_source(path)?;
    let size = std::fs::metadata(path)
        .map_err(|error| {
            CoreError::new(
                CoreErrorKind::SourceUnavailable,
                format!("cannot stat {}: {error}", path.display()),
            )
        })?
        .len();
    let mtime = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        });
    let now = crate::util::now_millis();
    let hash_text = content_hash.to_string();
    let path_text = path.to_string_lossy().into_owned();

    // Probe outside the transaction: the audio engine must never block the
    // catalog mutex. A rejected source still imports without metadata.
    let probe = echo_bridge::probe(path)
        .ok()
        .filter(|result| result.has_audio);

    catalog.with_transaction(|transaction| {
        // Relink: a previously missing asset with this hash comes back.
        let relinked = echo_catalog::relink_asset_by_hash(transaction, &hash_text, path)?;
        if !relinked {
            let existing = find_by_content_hash(transaction, content_hash)?;
            if matches!(existing, AssetLookup::NotFound) {
                echo_catalog::register_asset(
                    transaction,
                    &echo_catalog::AssetRegistrationInput {
                        content_hash,
                        path,
                        size_bytes: size,
                        codec: probe.as_ref().map(|p| p.codec_name.as_str()),
                        duration_millis: probe
                            .as_ref()
                            .filter(|p| p.duration_millis > 0)
                            .map(|p| p.duration_millis),
                        recorded_at_millis: probe
                            .as_ref()
                            .filter(|p| p.recorded_at_millis > 0)
                            .map(|p| p.recorded_at_millis),
                        imported_at_millis: now,
                    },
                )?;
                let asset = find_by_content_hash(transaction, content_hash)?;
                if let AssetLookup::Found(asset) = asset {
                    if let Some(probe) = &probe {
                        echo_catalog::record_source_metadata(
                            transaction,
                            asset.id,
                            &echo_catalog::SourceMetadata {
                                container_format: probe.container_format.clone(),
                                sample_rate: probe.sample_rate,
                                channel_count: probe.channel_count,
                                entries: probe
                                    .metadata
                                    .iter()
                                    .map(|entry| echo_catalog::SourceMetadataEntry {
                                        key: entry.key.clone(),
                                        value: entry.value.clone(),
                                    })
                                    .collect(),
                            },
                            (probe.recorded_at_millis > 0).then_some(probe.recorded_at_millis),
                        )?;
                    }
                    // Import remains model-independent: it only persists
                    // derivation intents. Structural waveform work is claimed
                    // before progressive ASR by the queue policy.
                    enqueue_job(
                        transaction,
                        &format!("waveform-{}", asset.id),
                        JobKind::AnalyzeWaveform,
                        &asset_id_payload(&asset.id),
                        now,
                    )?;
                    analysis_queue::enqueue_transcription(transaction, asset.id, now)?;
                }
            }
        }
        upsert_journal(transaction, &path_text, size, mtime, &hash_text)?;
        Ok(())
    })
}

fn asset_id_of(payload: &serde_json::Value) -> Result<echo_domain::AssetId, CoreError> {
    let id = payload
        .get("asset_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CoreError::new(CoreErrorKind::Other, "job payload lacks asset_id"))?;
    id.parse::<echo_domain::AssetId>()
        .map_err(|error| CoreError::new(CoreErrorKind::Other, format!("bad asset id: {error}")))
}

fn asset_id_payload(id: &echo_domain::AssetId) -> serde_json::Value {
    serde_json::json!({ "asset_id": id.to_string() })
}

fn source_path_of(
    catalog: &Catalog,
    asset_id: &echo_domain::AssetId,
) -> Result<PathBuf, CoreError> {
    catalog.with_transaction(|transaction| match find_by_id(transaction, *asset_id) {
        Ok(AssetLookup::Found(asset)) => Ok(asset.original.path),
        Ok(AssetLookup::NotFound) => Err(CoreError::new(
            CoreErrorKind::Other,
            format!("asset {asset_id} not found"),
        )),
        Err(error) => Err(CoreError::from(error)),
    })
}

fn newest_transcript(
    catalog: &Catalog,
    asset_id: echo_domain::AssetId,
) -> Result<crate::TranscriptPayload, CoreError> {
    let records = catalog.with_transaction(|transaction| {
        echo_catalog::query_analysis(transaction, asset_id)
            .map_err(|error| CoreError::new(CoreErrorKind::Other, error.to_string()))
    })?;
    records
        .iter()
        .find(|record| record.kind == echo_domain::AnalysisKind::Transcript)
        .and_then(|record| serde_json::from_value(record.value.clone()).ok())
        .ok_or_else(|| {
            CoreError::new(
                CoreErrorKind::InferenceRejected,
                "alignment requires transcript evidence",
            )
        })
}

fn record_inference_submitting(
    catalog: &Catalog,
    job: &ClaimedJob,
    asset_id: echo_domain::AssetId,
    intent: &str,
) -> Result<(), CoreError> {
    catalog
        .with_transaction(|transaction| {
            upsert_inference_run(
                transaction,
                &UpsertInferenceRun {
                    local_job_id: &job.id,
                    asset_id,
                    intent,
                    runtime_job_id: None,
                    contract_version: crate::EXPECTED_CONTRACT_VERSION,
                    state: InferenceRunState::Submitting,
                    http_status: None,
                    error_code: None,
                    snapshot: None,
                    updated_at_millis: crate::util::now_millis(),
                },
            )
        })
        .map_err(CoreError::from)
}

fn record_inference_success(
    catalog: &Catalog,
    job: &ClaimedJob,
    asset_id: echo_domain::AssetId,
    provenance: &crate::RuntimeProvenance,
) -> Result<(), CoreError> {
    let snapshot = serde_json::to_value(&provenance.job).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode Runtime provenance: {error}"),
        )
    })?;
    catalog
        .with_transaction(|transaction| {
            upsert_inference_run(
                transaction,
                &UpsertInferenceRun {
                    local_job_id: &job.id,
                    asset_id,
                    intent: &provenance.job.intent,
                    runtime_job_id: Some(&provenance.job.id),
                    contract_version: &provenance.contract_version,
                    state: InferenceRunState::Succeeded,
                    http_status: Some(200),
                    error_code: None,
                    snapshot: Some(&snapshot),
                    updated_at_millis: crate::util::now_millis(),
                },
            )
        })
        .map_err(CoreError::from)
}

fn record_inference_failure(
    catalog: &Catalog,
    job: &ClaimedJob,
    asset_id: echo_domain::AssetId,
    intent: &str,
    error: &crate::InferRuntimeError,
) -> Result<(), CoreError> {
    let state = match error.kind {
        crate::InferRuntimeErrorKind::Cancelled => InferenceRunState::Cancelled,
        crate::InferRuntimeErrorKind::Deadline => InferenceRunState::Expired,
        _ => InferenceRunState::Failed,
    };
    catalog
        .with_transaction(|transaction| {
            upsert_inference_run(
                transaction,
                &UpsertInferenceRun {
                    local_job_id: &job.id,
                    asset_id,
                    intent,
                    runtime_job_id: None,
                    contract_version: crate::EXPECTED_CONTRACT_VERSION,
                    state,
                    http_status: error.http_status,
                    error_code: Some(&error.code),
                    snapshot: None,
                    updated_at_millis: crate::util::now_millis(),
                },
            )
        })
        .map_err(CoreError::from)
}

fn record_inference_ingestion_failure(
    catalog: &Catalog,
    job: &ClaimedJob,
    asset_id: echo_domain::AssetId,
    provenance: &crate::RuntimeProvenance,
    validation_code: &str,
) -> Result<(), CoreError> {
    let snapshot = serde_json::to_value(&provenance.job).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode Runtime provenance: {error}"),
        )
    })?;
    let error_code = format!("contextual_{validation_code}");
    catalog
        .with_transaction(|transaction| {
            upsert_inference_run(
                transaction,
                &UpsertInferenceRun {
                    local_job_id: &job.id,
                    asset_id,
                    intent: &provenance.job.intent,
                    runtime_job_id: Some(&provenance.job.id),
                    contract_version: &provenance.contract_version,
                    state: InferenceRunState::Failed,
                    http_status: Some(200),
                    error_code: Some(&error_code),
                    snapshot: Some(&snapshot),
                    updated_at_millis: crate::util::now_millis(),
                },
            )
        })
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests;
