//! Waveform artifact workflow: build the pyramid through the audio engine
//! and publish it into the content-addressed cache.
//!
//! The cache payload is a rebuildable artifact, never a fact: the pyramid can
//! be regenerated from the immutable original at any time. The payload schema
//! is owned here.

use std::path::Path;

use echo_cache::{BlobRole, open_blob_store, put_blob};
use echo_domain::ContentHash;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

const WAVEFORM_ARTIFACT_SCHEMA: u32 = 1;

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
    Ok(WaveformArtifact {
        content_hash: blob.content_hash,
        canonical_sample_rate: payload.canonical_sample_rate,
        level_count: payload.levels.len(),
        bucket_count,
    })
}
