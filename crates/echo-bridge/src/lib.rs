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

pub use error::{BridgeError, BridgeErrorKind};

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

    struct FfiPreparedImpulseResponse {
        preparation_version: u32,
        source_sample_rate: u32,
        channel_count: u32,
        source_frame_count: u64,
        prepared_frame_count: u64,
        avcodec_version: u32,
        swresample_version: u32,
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
        fn prepare_impulse_response_bridge(
            source_path: &str,
            output_path: &str,
            layout: u8,
        ) -> Result<FfiPreparedImpulseResponse>;
    }
}

/// Explicit interpretation requested for one source WAV.
///
/// Four-channel WAV files are never inferred to be true stereo. The caller
/// must opt into the frozen LL/LR/RL/RR plane contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ImpulseResponsePreparationLayout {
    MonoOrStereo = 0,
    TrueStereoLlLrRlRr = 1,
}

/// Evidence for one canonical 48 kHz impulse-response artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedImpulseResponse {
    pub preparation_version: u32,
    pub source_sample_rate: u32,
    pub channel_count: u32,
    pub source_frame_count: u64,
    pub prepared_frame_count: u64,
    pub avcodec_version: u32,
    pub swresample_version: u32,
    pub size_bytes: u64,
}

/// Validates a mono or stereo local WAV and writes canonical planar float32
/// at 48 kHz using the stable v1 preparation contract.
///
/// # Errors
///
/// Returns [`BridgeErrorKind::EngineRejected`] for unsupported WAV structure,
/// sample layout, duration, non-finite/silent content, or I/O failures.
pub fn prepare_impulse_response(
    source: &Path,
    output: &Path,
) -> Result<PreparedImpulseResponse, BridgeError> {
    prepare_impulse_response_with_layout(
        source,
        output,
        ImpulseResponsePreparationLayout::MonoOrStereo,
    )
}

/// Validates a bounded local WAV under an explicit plane interpretation and
/// writes canonical planar float32 at 48 kHz.
///
/// The output is a versioned portable preparation artifact, not a runtime FFT
/// bank. The caller supplies a temporary output path and publishes it only
/// after hashing and source-identity checks succeed.
///
/// # Errors
///
/// Returns [`BridgeErrorKind::EngineRejected`] for unsupported WAV structure,
/// sample layout, duration, non-finite/silent content, or I/O failures.
pub fn prepare_impulse_response_with_layout(
    source: &Path,
    output: &Path,
    layout: ImpulseResponsePreparationLayout,
) -> Result<PreparedImpulseResponse, BridgeError> {
    let source = native_path(source)?;
    let output = native_path(output)?;
    let wire = ffi::prepare_impulse_response_bridge(&source, &output, layout as u8)?;
    Ok(PreparedImpulseResponse {
        preparation_version: wire.preparation_version,
        source_sample_rate: wire.source_sample_rate,
        channel_count: wire.channel_count,
        source_frame_count: wire.source_frame_count,
        prepared_frame_count: wire.prepared_frame_count,
        avcodec_version: wire.avcodec_version,
        swresample_version: wire.swresample_version,
        size_bytes: wire.size_bytes,
    })
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
