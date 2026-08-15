#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace echo::audio {

/// Rebuildable logarithmic-magnitude overview of one immutable original.
///
/// Rows progress from low to high frequency. `magnitudes` is row-major by
/// time column, normalized to `0..=255`; it is presentation evidence only,
/// never authored adjustment data.
struct SpectrogramOverview {
    std::uint32_t canonical_sample_rate = 0;
    std::uint32_t window_frames = 0;
    std::uint32_t hop_frames = 0;
    std::uint32_t time_columns = 0;
    std::uint32_t frequency_bins = 0;
    std::vector<std::uint8_t> magnitudes;
};

/// Streams an original into a bounded STFT overview. `max_time_columns` and
/// `frequency_bins` define the returned payload cap rather than persisted
/// audio intent; long sources are deterministically max-pooled in time.
///
/// @throws std::runtime_error when the source cannot be decoded.
SpectrogramOverview build_spectrogram_overview(
    const std::string& path,
    std::uint32_t max_time_columns,
    std::uint32_t frequency_bins
);

} // namespace echo::audio
