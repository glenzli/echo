#pragma once

#include <cstdint>
#include <stop_token>
#include <string>
#include <vector>

namespace echo::audio {

/// Display-only analysis of an exact Original-time viewport. Never edit intent.
struct SpectrogramDetailRequest {
    std::uint64_t start_millis = 0;
    std::uint64_t end_millis = 0;
    double low_hertz = 20;
    double high_hertz = 24'000;
    bool logarithmic = true;
    std::uint32_t window_frames = 8'192;
    std::uint32_t columns = 1'024;
    std::uint32_t rows = 384;
    float floor_decibels = -96;
    float ceiling_decibels = 0;
};

struct SpectrogramDetail {
    std::uint32_t columns = 0;
    std::uint32_t rows = 0;
    /// Column-major, ascending frequency; peak magnitude across channels.
    std::vector<std::uint8_t> magnitudes;
};

/// Bounded streaming STFT using the canonical decoder and exact time buckets.
/// Stereo channels are analyzed separately to preserve out-of-phase evidence.
SpectrogramDetail build_spectrogram_detail(
    const std::string& path,
    const SpectrogramDetailRequest& request,
    std::stop_token cancellation = {}
);

} // namespace echo::audio
