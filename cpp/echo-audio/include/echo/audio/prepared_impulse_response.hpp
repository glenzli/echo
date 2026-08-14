#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace echo::audio {

enum class PreparedImpulseLayout : std::uint8_t {
    Mono = 0,
    StereoParallel = 1,
    TrueStereoLlLrRlRr = 2,
};

/// Fully validated planar samples ready for offline runtime-bank construction.
///
/// This owner reads the portable preparation artifact only. Content-addressed
/// cache verification and source/provenance lookup remain Rust lifecycle work;
/// FFT allocation remains `ConvolutionSpaceProcessor` construction work.
struct LoadedPreparedImpulseResponse {
    std::uint32_t preparation_version = 0;
    std::uint32_t source_sample_rate = 0;
    std::uint64_t source_frame_count = 0;
    std::uint32_t avcodec_version = 0;
    std::uint32_t swresample_version = 0;
    PreparedImpulseLayout layout = PreparedImpulseLayout::Mono;
    /// LL for stereo layouts, or the shared diagonal impulse for Mono.
    std::vector<float> left;
    /// RR for stereo layouts. Empty for Mono.
    std::vector<float> right;
    /// L input -> R output. Present only for TrueStereoLlLrRlRr.
    std::vector<float> left_to_right;
    /// R input -> L output. Present only for TrueStereoLlLrRlRr.
    std::vector<float> right_to_left;
};

/// Loads one exact `ECHOIR01` artifact: v1 mono/stereo-parallel or v2
/// true-stereo LL/LR/RL/RR. Other version/channel combinations and malformed,
/// truncated, non-finite, silent, or non-48 kHz data are rejected.
///
/// This is a worker-thread operation and may allocate.
///
/// @throws std::runtime_error when the artifact is unusable.
[[nodiscard]] LoadedPreparedImpulseResponse load_prepared_impulse_response(const std::string& path);

} // namespace echo::audio
