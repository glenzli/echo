//! Safe Rust API over the C++ audio engine (`cpp/echo-audio`).
//!
//! This crate owns the coarse CXX boundary: probe metadata and waveform
//! extraction. The generated wire declaration stays centralized and auditable
//! in [`ffi`]; safe callers go through [`probe`] and [`waveform`].
//!
//! The real-time audio path (playback, DSP) will later own a separate
//! narrow boundary so that no AI work ever crosses an audio callback.

mod error;
pub mod waveform;

use std::path::Path;

use error::{BridgeError, BridgeErrorKind};

#[cxx::bridge(namespace = "echo::bridge")]
mod ffi {
    struct FfiAudioProbe {
        has_audio: bool,
        codec_name: String,
        container_format: String,
        sample_rate: u32,
        channel_count: u32,
        duration_millis: u64,
    }

    struct FfiWaveformLevel {
        mins: Vec<f32>,
        maxs: Vec<f32>,
        samples_per_bucket: u32,
    }

    struct FfiWaveform {
        canonical_sample_rate: u32,
        levels: Vec<FfiWaveformLevel>,
    }

    unsafe extern "C++" {
        include!("src/bridge/cxx_bridge.hpp");

        fn probe_audio(path: &str) -> Result<FfiAudioProbe>;
        fn build_waveform_bridge(path: &str, max_levels: u32) -> Result<FfiWaveform>;
    }
}

/// Probe result for one source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProbe {
    pub has_audio: bool,
    pub codec_name: String,
    pub container_format: String,
    pub sample_rate: u32,
    pub channel_count: u32,
    pub duration_millis: u64,
}

/// Probes a source file without decoding samples.
///
/// # Errors
///
/// Returns [`BridgeErrorKind::EngineRejected`] when the engine cannot open
/// or probe the source.
pub fn probe(path: &Path) -> Result<AudioProbe, BridgeError> {
    let text = native_path(path)?;
    let wire = ffi::probe_audio(&text)?;
    Ok(AudioProbe {
        has_audio: wire.has_audio,
        codec_name: wire.codec_name,
        container_format: wire.container_format,
        sample_rate: wire.sample_rate,
        channel_count: wire.channel_count,
        duration_millis: wire.duration_millis,
    })
}

fn native_path(path: &Path) -> Result<String, BridgeError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        BridgeError::new(
            BridgeErrorKind::EngineRejected,
            format!("path is not valid UTF-8: {}", path.display()),
        )
    })
}
