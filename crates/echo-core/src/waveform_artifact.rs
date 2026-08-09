//! Waveform artifact workflow: build the pyramid through the audio engine
//! and publish it into the content-addressed cache.
//!
//! The cache payload is a rebuildable artifact, never a fact: the pyramid can
//! be regenerated from the immutable original at any time. The payload schema
//! is owned here.

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

const WAVEFORM_ARTIFACT_SCHEMA: u32 = 1;

/// Upper bound for a waveform pyramid read. A few hours of base-level peaks
/// remain well below this limit.
const MAX_WAVEFORM_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;

/// Stable payload schema of a cached waveform artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformArtifactPayload {
    pub schema: u32,
    pub canonical_sample_rate: u32,
    pub levels: Vec<WaveformArtifactLevel>,
}

/// One pyramid level inside the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformArtifactLevel {
    pub samples_per_bucket: u32,
    pub mins: Vec<f32>,
    pub maxs: Vec<f32>,
}

/// Durable reference to a cached waveform artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveformArtifact {
    pub content_hash: ContentHash,
    pub canonical_sample_rate: u32,
    pub level_count: usize,
    pub bucket_count: u64,
}

/// Builds the waveform pyramid for `source` and publishes it into the cache
/// under its content hash. The original file is never modified.
///
/// # Panics
///
/// Panics when the base-level bucket count exceeds `u64`.
///
/// # Errors
///
/// Returns [`CoreErrorKind::AudioEngineRejected`] when the source cannot be
/// decoded and [`CoreErrorKind::Other`] when the cache write fails.
pub fn build_and_cache_waveform(
    source: &Path,
    cache_root: &Path,
    max_levels: u32,
) -> Result<WaveformArtifact, CoreError> {
    build_waveform_artifact(source, cache_root, max_levels).map(|built| built.artifact)
}

/// Resolves an asset's current waveform artifact, rebuilding and publishing
/// a new reference when the cache entry is missing, corrupt, or from an
/// incompatible payload schema.
///
/// # Errors
///
/// Returns a core failure when the catalog cannot be read or the original
/// cannot be decoded into a replacement artifact.
pub fn load_or_build_waveform(
    catalog: &Catalog,
    asset_id: AssetId,
    source: &Path,
    cache_root: &Path,
    max_levels: u32,
) -> Result<WaveformArtifactPayload, CoreError> {
    let reference = catalog.with_transaction(|transaction| {
        find_derived_artifact(
            transaction,
            asset_id,
            DerivedArtifactKind::WaveformPyramid,
            WAVEFORM_ARTIFACT_SCHEMA,
        )
    })?;
    if let Some(reference) = reference {
        if let Ok(payload) =
            read_waveform_payload(cache_root, reference.content_hash, reference.size_bytes)
        {
            return Ok(payload);
        }
        catalog.with_transaction(|transaction| {
            remove_derived_artifact(
                transaction,
                asset_id,
                DerivedArtifactKind::WaveformPyramid,
                WAVEFORM_ARTIFACT_SCHEMA,
            )
        })?;
    }

    let built = build_waveform_artifact(source, cache_root, max_levels)?;
    catalog.with_transaction(|transaction| {
        upsert_derived_artifact(
            transaction,
            &DerivedArtifactRecord {
                asset_id,
                kind: DerivedArtifactKind::WaveformPyramid,
                schema_version: WAVEFORM_ARTIFACT_SCHEMA,
                content_hash: built.artifact.content_hash,
                size_bytes: built.size_bytes,
                created_at_millis: crate::util::now_millis(),
            },
        )
    })?;
    Ok(built.payload)
}

struct BuiltWaveformArtifact {
    artifact: WaveformArtifact,
    payload: WaveformArtifactPayload,
    size_bytes: u64,
}

fn build_waveform_artifact(
    source: &Path,
    cache_root: &Path,
    max_levels: u32,
) -> Result<BuiltWaveformArtifact, CoreError> {
    let waveform = echo_bridge::waveform::build_waveform(source, max_levels)
        .map_err(|error| CoreError::new(CoreErrorKind::AudioEngineRejected, error.message))?;
    let payload = WaveformArtifactPayload {
        schema: WAVEFORM_ARTIFACT_SCHEMA,
        canonical_sample_rate: waveform.canonical_sample_rate,
        levels: waveform
            .levels
            .into_iter()
            .map(|level| WaveformArtifactLevel {
                samples_per_bucket: level.samples_per_bucket,
                mins: level.mins,
                maxs: level.maxs,
            })
            .collect(),
    };
    let encoded = serde_json::to_vec(&payload).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode waveform artifact: {error}"),
        )
    })?;
    let store = open_blob_store(cache_root).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot open cache at {}: {error}", cache_root.display()),
        )
    })?;
    let (blob, _) = put_blob(&store, BlobRole::WaveformPyramid, &encoded).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot store waveform artifact: {error}"),
        )
    })?;
    let bucket_count = payload.levels.first().map_or(0, |level| {
        u64::try_from(level.mins.len()).expect("bucket count fits u64")
    });
    Ok(BuiltWaveformArtifact {
        artifact: WaveformArtifact {
            content_hash: blob.content_hash,
            canonical_sample_rate: payload.canonical_sample_rate,
            level_count: payload.levels.len(),
            bucket_count,
        },
        payload,
        size_bytes: blob.size_bytes,
    })
}

fn read_waveform_payload(
    cache_root: &Path,
    content_hash: ContentHash,
    expected_size: u64,
) -> Result<WaveformArtifactPayload, CoreError> {
    let store = open_blob_store(cache_root).map_err(|error| {
        CoreError::new(CoreErrorKind::Other, format!("cannot open cache: {error}"))
    })?;
    let bytes =
        read_verified(&store, content_hash, MAX_WAVEFORM_ARTIFACT_BYTES).map_err(|error| {
            if error.kind == CacheErrorKind::Corrupt {
                let _ = quarantine_corrupt(&store, content_hash, expected_size);
            }
            CoreError::new(
                CoreErrorKind::Other,
                format!("cannot read waveform artifact: {error}"),
            )
        })?;
    let payload: WaveformArtifactPayload = serde_json::from_slice(&bytes).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot decode waveform artifact: {error}"),
        )
    })?;
    if payload.schema != WAVEFORM_ARTIFACT_SCHEMA
        || payload
            .levels
            .iter()
            .any(|level| level.mins.len() != level.maxs.len())
    {
        return Err(CoreError::new(
            CoreErrorKind::Other,
            "waveform artifact payload schema is incompatible",
        ));
    }
    Ok(payload)
}

#[cfg(test)]
mod tests;
