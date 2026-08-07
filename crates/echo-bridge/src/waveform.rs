//! Waveform pyramid extraction through the audio engine.

use std::path::Path;

use crate::error::{BridgeError, BridgeErrorKind};

/// One pyramid level over the canonical mono mixdown.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveformLevel {
    /// Per-bucket minimum samples (interleaved with `maxs`).
    pub mins: Vec<f32>,
    /// Per-bucket maximum samples.
    pub maxs: Vec<f32>,
    /// Samples per bucket at this level.
    pub samples_per_bucket: u32,
}

/// A waveform pyramid, finest level first.
#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    pub canonical_sample_rate: u32,
    pub levels: Vec<WaveformLevel>,
}

/// Builds a waveform pyramid by streaming the source to the canonical mono
/// mixdown; the original file is never modified.
///
/// `max_levels` bounds the pyramid height (at least 1).
///
/// # Errors
///
/// Returns [`BridgeErrorKind::EngineRejected`] when the source cannot be
/// decoded.
pub fn build_waveform(path: &Path, max_levels: u32) -> Result<Waveform, BridgeError> {
    if max_levels == 0 {
        return Err(BridgeError::new(
            BridgeErrorKind::EngineRejected,
            "max_levels must be positive",
        ));
    }
    let text = crate::native_path(path)?;
    let wire = crate::ffi::build_waveform_bridge(&text, max_levels)?;
    Ok(Waveform {
        canonical_sample_rate: wire.canonical_sample_rate,
        levels: wire
            .levels
            .into_iter()
            .map(|level| WaveformLevel {
                mins: level.mins,
                maxs: level.maxs,
                samples_per_bucket: level.samples_per_bucket,
            })
            .collect(),
    })
}
