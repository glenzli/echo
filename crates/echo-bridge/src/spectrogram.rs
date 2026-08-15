//! Bounded spectrogram overview extraction through the audio engine.

use std::path::Path;

use crate::{error::BridgeError, native_path};

/// Read-only logarithmic-magnitude overview for one immutable original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpectrogramOverview {
    pub canonical_sample_rate: u32,
    pub window_frames: u32,
    pub hop_frames: u32,
    pub time_columns: u32,
    pub frequency_bins: u32,
    pub magnitudes: Vec<u8>,
}

/// Builds a bounded spectrogram overview without modifying the original.
pub fn build_spectrogram_overview(
    path: &Path,
    max_time_columns: u32,
    frequency_bins: u32,
) -> Result<SpectrogramOverview, BridgeError> {
    let text = native_path(path)?;
    let wire =
        crate::ffi::build_spectrogram_overview_bridge(&text, max_time_columns, frequency_bins)?;
    Ok(SpectrogramOverview {
        canonical_sample_rate: wire.canonical_sample_rate,
        window_frames: wire.window_frames,
        hop_frames: wire.hop_frames,
        time_columns: wire.time_columns,
        frequency_bins: wire.frequency_bins,
        magnitudes: wire.magnitudes,
    })
}
