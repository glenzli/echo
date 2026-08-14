#pragma once

#include <cstdint>
#include <string>

namespace echo::audio {

inline constexpr std::uint32_t kPreparedImpulseResponseSampleRate = 48000;
inline constexpr std::uint32_t kPreparedImpulseResponseVersion = 1;
inline constexpr std::uint32_t kPreparedTrueStereoImpulseResponseVersion = 2;
inline constexpr std::uint32_t kPreparedImpulseResponseHeaderBytes = 64;

/// Explicit interpretation requested for one source WAV.
///
/// Four-channel WAVEFORMATEXTENSIBLE files describe speaker positions, not an
/// impulse-response matrix, so they are never inferred automatically. The
/// true-stereo intent freezes source planes FL/FR/BL/BR as LL/LR/RL/RR, where
/// each pair names input -> output.
enum class ImpulseResponsePreparationLayout : std::uint8_t {
    AutoMonoOrStereo = 0,
    TrueStereoLlLrRlRr = 1,
};

/// Evidence for one deterministic local-WAV preparation.
struct PreparedImpulseResponseResult {
    std::uint32_t preparation_version = 0;
    std::uint32_t source_sample_rate = 0;
    std::uint32_t channel_count = 0;
    std::uint64_t source_frame_count = 0;
    std::uint64_t prepared_frame_count = 0;
    std::uint32_t avcodec_version = 0;
    std::uint32_t swresample_version = 0;
    std::uint64_t size_bytes = 0;

    bool operator==(const PreparedImpulseResponseResult&) const = default;
};

/// Validates and decodes one bounded local RIFF/WAVE source, resamples it to
/// canonical 48 kHz planar float32, and writes a portable versioned artifact.
///
/// The default v1 path accepts PCM16/24/32 or IEEE float32, mono or stereo,
/// 8--192 kHz. Explicit TrueStereoLlLrRlRr accepts only four-channel
/// WAVEFORMATEXTENSIBLE with the exact quad mask 0x33 and writes v2 planar
/// LL/LR/RL/RR. It rejects RF64, compressed WAV, ambiguous layouts, non-finite
/// samples, digital silence, and results longer than five seconds. Amplitude,
/// leading silence, and the full accepted tail are preserved.
///
/// This is an offline worker operation and may allocate. The caller supplies a
/// temporary output path and owns removal after any exception.
///
/// @throws std::runtime_error when input validation, decoding, resampling, or
/// output publication fails.
[[nodiscard]] PreparedImpulseResponseResult prepare_impulse_response(
    const std::string& source_path,
    const std::string& output_path,
    ImpulseResponsePreparationLayout layout = ImpulseResponsePreparationLayout::AutoMonoOrStereo
);

} // namespace echo::audio
