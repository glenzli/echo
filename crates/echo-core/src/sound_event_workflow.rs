//! Background orchestration for Runtime sound-event detection.
//!
//! Admission happens only after valid empty ASR. This owner selects bounded
//! Original or existing long-audio proxy sources, records Runtime lifecycle,
//! and commits one complete `AudioEvents` envelope before updating rebuildable
//! browse and semantic projections.

use std::path::PathBuf;

use echo_catalog::{AssetLookup, Catalog, ClaimedJob, find_by_id, list_long_audio_segments};

use crate::{
    AudioEventChunk, AudioEventDetectionIntent, AudioEventsEvidence, CoreError, CoreErrorKind,
    InferRuntimeClient, WorkerConfig,
};

pub(crate) fn dispatch(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    let asset_id = asset_id_of(&job.payload)?;
    let asset =
        catalog.with_transaction(|transaction| match find_by_id(transaction, asset_id) {
            Ok(AssetLookup::Found(asset)) => Ok(asset),
            Ok(AssetLookup::NotFound) => Err(CoreError::new(
                CoreErrorKind::Other,
                format!("asset {asset_id} not found"),
            )),
            Err(error) => Err(CoreError::from(error)),
        })?;
    super::worker::record_inference_submitting(
        catalog,
        job,
        asset_id,
        crate::AUDIO_EVENT_DETECTION_INTENT,
    )?;
    let sources = bounded_sources(catalog, config, &asset)?;
    let client = InferRuntimeClient::new(config.infer_runtime.clone());
    let mut chunks = Vec::with_capacity(sources.len());
    for source in sources {
        let detection =
            match client.detect_audio_events(&source.path, &AudioEventDetectionIntent::default()) {
                Ok(detection) => detection,
                Err(error) => {
                    super::worker::record_inference_failure(
                        catalog,
                        job,
                        asset_id,
                        crate::AUDIO_EVENT_DETECTION_INTENT,
                        &error,
                    )?;
                    return Err(error.into());
                }
            };
        chunks.push(AudioEventChunk {
            source_start_seconds: source.start_seconds,
            source_end_seconds: source.start_seconds + detection.coverage.input_duration_seconds,
            detection,
        });
    }
    let evidence = AudioEventsEvidence {
        schema_version: crate::AUDIO_EVENTS_SCHEMA_VERSION,
        chunks,
    };
    let runtime = &evidence
        .chunks
        .last()
        .ok_or_else(|| {
            CoreError::new(
                CoreErrorKind::InferenceRejected,
                "sound-event analysis has no bounded source",
            )
        })?
        .detection
        .runtime;
    super::worker::record_inference_success(catalog, job, asset_id, runtime)?;
    if crate::record_audio_events(catalog, asset_id, &evidence)? {
        crate::semantic_search::enqueue_current_document(
            catalog,
            asset_id,
            crate::util::now_millis(),
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct BoundedSource {
    start_seconds: f64,
    path: PathBuf,
}

fn bounded_sources(
    catalog: &Catalog,
    config: &WorkerConfig,
    asset: &echo_domain::AudioAsset,
) -> Result<Vec<BoundedSource>, CoreError> {
    if !crate::long_audio::requires_segmentation(asset) {
        return Ok(vec![BoundedSource {
            start_seconds: 0.0,
            path: asset.original.path.clone(),
        }]);
    }
    let store = echo_cache::open_blob_store(&config.cache_root).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot open analysis proxy cache: {error}"),
        )
    })?;
    let segments = catalog.with_transaction(|transaction| {
        list_long_audio_segments(transaction, asset.id, crate::LONG_AUDIO_PLAN_VERSION)
            .map_err(CoreError::from)
    })?;
    if segments.is_empty() {
        return Err(CoreError::new(
            CoreErrorKind::InferenceRejected,
            "long audio has no bounded analysis proxy plan",
        ));
    }
    segments
        .into_iter()
        .map(|segment| {
            let proxy = segment.proxy.ok_or_else(|| {
                CoreError::new(
                    CoreErrorKind::InferenceRejected,
                    "long audio analysis proxy is not available",
                )
            })?;
            let path = echo_cache::verify_blob(&store, proxy.content_hash, proxy.size_bytes)
                .map_err(|error| {
                    CoreError::new(
                        CoreErrorKind::Other,
                        format!("long audio analysis proxy is corrupt: {error}"),
                    )
                })?;
            #[allow(clippy::cast_precision_loss)]
            let start_seconds = segment.start_millis as f64 / 1_000.0;
            Ok(BoundedSource {
                start_seconds,
                path,
            })
        })
        .collect()
}

fn asset_id_of(payload: &serde_json::Value) -> Result<echo_domain::AssetId, CoreError> {
    let id = payload
        .get("asset_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CoreError::new(CoreErrorKind::Other, "event job lacks asset_id"))?;
    id.parse::<echo_domain::AssetId>()
        .map_err(|error| CoreError::new(CoreErrorKind::Other, format!("bad asset id: {error}")))
}

#[cfg(test)]
mod tests;
