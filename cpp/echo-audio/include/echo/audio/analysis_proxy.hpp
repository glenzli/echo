#pragma once

#include <cstdint>
#include <string>

namespace echo::audio {

/// Bounded analysis WAV produced from one immutable Original time range.
struct AnalysisProxyResult {
    std::uint32_t sample_rate = 0;
    std::uint32_t channel_count = 0;
    std::uint64_t frame_count = 0;
    std::uint64_t size_bytes = 0;
};

/// Streams one time range to a mono 16 kHz, 16-bit PCM WAV. The source is
/// never modified and decoded samples are written incrementally.
///
/// @throws std::runtime_error when the range is invalid or cannot be decoded.
AnalysisProxyResult build_analysis_proxy(
    const std::string& source_path,
    const std::string& output_path,
    std::uint64_t start_millis,
    std::uint64_t end_millis
);

} // namespace echo::audio
