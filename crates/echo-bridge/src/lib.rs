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
    struct FfiAudioMetadataEntry {
        key: String,
        value: String,
    }

    struct FfiAudioProbe {
        has_audio: bool,
        codec_name: String,
        container_format: String,
        sample_rate: u32,
        channel_count: u32,
        duration_millis: u64,
        recorded_at_millis: i64,
        metadata: Vec<FfiAudioMetadataEntry>,
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

    struct FfiAnalysisProxy {
        sample_rate: u32,
        channel_count: u32,
        frame_count: u64,
        size_bytes: u64,
    }

    unsafe extern "C++" {
        include!("src/bridge/cxx_bridge.hpp");

        fn probe_audio(path: &str) -> Result<FfiAudioProbe>;
        fn build_waveform_bridge(path: &str, max_levels: u32) -> Result<FfiWaveform>;
        fn build_analysis_proxy_bridge(
            source_path: &str,
            output_path: &str,
            start_millis: u64,
            end_millis: u64,
        ) -> Result<FfiAnalysisProxy>;
    }
}

/// Metadata of one bounded analysis proxy written by the audio engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisProxy {
    pub sample_rate: u32,
    pub channel_count: u32,
    pub frame_count: u64,
    pub size_bytes: u64,
}

/// Streams one source time range to a mono 16 kHz PCM16 WAV at `output`.
///
/// # Errors
///
/// Returns an engine failure when paths are invalid or the range cannot be
/// decoded. A failed output is owned by the caller and may be discarded.
pub fn build_analysis_proxy(
    source: &Path,
    output: &Path,
    start_millis: u64,
    end_millis: u64,
) -> Result<AnalysisProxy, BridgeError> {
    let source = native_path(source)?;
    let output = native_path(output)?;
    let wire = ffi::build_analysis_proxy_bridge(&source, &output, start_millis, end_millis)?;
    Ok(AnalysisProxy {
        sample_rate: wire.sample_rate,
        channel_count: wire.channel_count,
        frame_count: wire.frame_count,
        size_bytes: wire.size_bytes,
    })
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
    pub recorded_at_millis: i64,
    pub metadata: Vec<AudioMetadataEntry>,
}

/// One bounded metadata entry from the original container or audio stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioMetadataEntry {
    pub key: String,
    pub value: String,
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
        recorded_at_millis: wire.recorded_at_millis,
        metadata: wire
            .metadata
            .into_iter()
            .map(|entry| AudioMetadataEntry {
                key: entry.key,
                value: entry.value,
            })
            .collect(),
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
