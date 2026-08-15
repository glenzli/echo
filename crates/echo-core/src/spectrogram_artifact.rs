//! Spectrogram overview artifact workflow for the spectral repair workspace.
//!
//! The overview is a bounded, rebuildable rendering aid derived solely from
//! the immutable original. It is deliberately separate from authored spectral
//! repair regions: it can be evicted and rebuilt without changing user work.

use std::path::Path;

use echo_cache::{
    BlobRole, CacheErrorKind, open_blob_store, put_blob, quarantine_corrupt, read_verified,
};
use echo_catalog::{
    Catalog, DerivedArtifactKind, DerivedArtifactRecord, find_derived_artifact,
    remove_derived_artifact, upsert_derived_artifact,
};
use echo_domain::{AssetId, ContentHash};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

const SPECTROGRAM_ARTIFACT_SCHEMA: u32 = 1;
const SPECTROGRAM_MAX_TIME_COLUMNS: u32 = 1024;
const SPECTROGRAM_FREQUENCY_BINS: u32 = 128;
const MAX_SPECTROGRAM_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;

/// Stable payload schema of a cached display-only spectrogram overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramArtifactPayload {
    pub schema: u32,
    pub canonical_sample_rate: u32,
    pub window_frames: u32,
    pub hop_frames: u32,
    pub time_columns: u32,
    pub frequency_bins: u32,
    /// Row-major magnitudes: `time_column * frequency_bins + frequency_bin`.
    pub magnitudes: Vec<u8>,
}

/// Durable reference to a cached spectrogram overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpectrogramArtifact {
    pub content_hash: ContentHash,
    pub canonical_sample_rate: u32,
    pub time_columns: u32,
    pub frequency_bins: u32,
}

/// Builds and caches the bounded overview for `source`.
///
/// The fixed resolution is part of this artifact schema. Changing it requires
/// a schema bump, which makes old cached payloads safely rebuild.
pub fn build_and_cache_spectrogram(
    source: &Path,
    cache_root: &Path,
) -> Result<SpectrogramArtifact, CoreError> {
    build_spectrogram_artifact(source, cache_root).map(|built| built.artifact)
}

/// Resolves an asset's overview, rebuilding it when its cache reference is
/// absent, corrupt, or incompatible with the current schema.
pub fn load_or_build_spectrogram(
    catalog: &Catalog,
    asset_id: AssetId,
    source: &Path,
    cache_root: &Path,
) -> Result<SpectrogramArtifactPayload, CoreError> {
    let reference = catalog.with_transaction(|transaction| {
        find_derived_artifact(
            transaction,
            asset_id,
            DerivedArtifactKind::SpectrogramOverview,
            SPECTROGRAM_ARTIFACT_SCHEMA,
        )
    })?;
    if let Some(reference) = reference {
        if let Ok(payload) =
            read_spectrogram_payload(cache_root, reference.content_hash, reference.size_bytes)
        {
            return Ok(payload);
        }
        catalog.with_transaction(|transaction| {
            remove_derived_artifact(
                transaction,
                asset_id,
                DerivedArtifactKind::SpectrogramOverview,
                SPECTROGRAM_ARTIFACT_SCHEMA,
            )
        })?;
    }

    let built = build_spectrogram_artifact(source, cache_root)?;
    catalog.with_transaction(|transaction| {
        upsert_derived_artifact(
            transaction,
            &DerivedArtifactRecord {
                asset_id,
                kind: DerivedArtifactKind::SpectrogramOverview,
                schema_version: SPECTROGRAM_ARTIFACT_SCHEMA,
                content_hash: built.artifact.content_hash,
                size_bytes: built.size_bytes,
                created_at_millis: crate::util::now_millis(),
            },
        )
    })?;
    Ok(built.payload)
}

struct BuiltSpectrogramArtifact {
    artifact: SpectrogramArtifact,
    payload: SpectrogramArtifactPayload,
    size_bytes: u64,
}

fn build_spectrogram_artifact(
    source: &Path,
    cache_root: &Path,
) -> Result<BuiltSpectrogramArtifact, CoreError> {
    let overview = echo_bridge::spectrogram::build_spectrogram_overview(
        source,
        SPECTROGRAM_MAX_TIME_COLUMNS,
        SPECTROGRAM_FREQUENCY_BINS,
    )
    .map_err(|error| CoreError::new(CoreErrorKind::AudioEngineRejected, error.message))?;
    let payload = SpectrogramArtifactPayload {
        schema: SPECTROGRAM_ARTIFACT_SCHEMA,
        canonical_sample_rate: overview.canonical_sample_rate,
        window_frames: overview.window_frames,
        hop_frames: overview.hop_frames,
        time_columns: overview.time_columns,
        frequency_bins: overview.frequency_bins,
        magnitudes: overview.magnitudes,
    };
    validate_payload(&payload)?;
    let encoded = serde_json::to_vec(&payload).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode spectrogram artifact: {error}"),
        )
    })?;
    let store = open_blob_store(cache_root).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot open cache at {}: {error}", cache_root.display()),
        )
    })?;
    let (blob, _) = put_blob(&store, BlobRole::SpectrogramOverview, &encoded).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot store spectrogram artifact: {error}"),
        )
    })?;
    Ok(BuiltSpectrogramArtifact {
        artifact: SpectrogramArtifact {
            content_hash: blob.content_hash,
            canonical_sample_rate: payload.canonical_sample_rate,
            time_columns: payload.time_columns,
            frequency_bins: payload.frequency_bins,
        },
        payload,
        size_bytes: blob.size_bytes,
    })
}

fn read_spectrogram_payload(
    cache_root: &Path,
    content_hash: ContentHash,
    expected_size: u64,
) -> Result<SpectrogramArtifactPayload, CoreError> {
    let store = open_blob_store(cache_root).map_err(|error| {
        CoreError::new(CoreErrorKind::Other, format!("cannot open cache: {error}"))
    })?;
    let bytes =
        read_verified(&store, content_hash, MAX_SPECTROGRAM_ARTIFACT_BYTES).map_err(|error| {
            if error.kind == CacheErrorKind::Corrupt {
                let _ = quarantine_corrupt(&store, content_hash, expected_size);
            }
            CoreError::new(
                CoreErrorKind::Other,
                format!("cannot read spectrogram artifact: {error}"),
            )
        })?;
    let payload: SpectrogramArtifactPayload = serde_json::from_slice(&bytes).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot decode spectrogram artifact: {error}"),
        )
    })?;
    validate_payload(&payload)?;
    Ok(payload)
}

fn validate_payload(payload: &SpectrogramArtifactPayload) -> Result<(), CoreError> {
    let expected_len = usize::try_from(payload.time_columns)
        .ok()
        .and_then(|time_columns| {
            usize::try_from(payload.frequency_bins)
                .ok()
                .and_then(|frequency_bins| time_columns.checked_mul(frequency_bins))
        });
    if payload.schema != SPECTROGRAM_ARTIFACT_SCHEMA
        || payload.canonical_sample_rate == 0
        || payload.window_frames == 0
        || payload.hop_frames == 0
        || payload.time_columns == 0
        || payload.time_columns > SPECTROGRAM_MAX_TIME_COLUMNS
        || payload.frequency_bins != SPECTROGRAM_FREQUENCY_BINS
        || expected_len != Some(payload.magnitudes.len())
    {
        return Err(CoreError::new(
            CoreErrorKind::Other,
            "spectrogram artifact payload schema is incompatible",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
