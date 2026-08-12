#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace echo::audio {

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
    std::vector<float> left;
    std::vector<float> right;
};

/// Loads one exact v1 `ECHOIR01` artifact and rejects malformed, truncated,
/// non-finite, silent, non-48 kHz, or unsupported-layout data.
///
/// This is a worker-thread operation and may allocate.
///
/// @throws std::runtime_error when the artifact is unusable.
[[nodiscard]] LoadedPreparedImpulseResponse load_prepared_impulse_response(const std::string& path);

} // namespace echo::audio
