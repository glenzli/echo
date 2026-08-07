#pragma once

#include <cstdint>
#include <vector>

namespace echo::audio {

/// One level of the waveform pyramid: per-bucket min/max peak pairs. Each
/// higher level halves the bucket count by pairing adjacent buckets
/// (min-of-mins, max-of-maxs), so every level is independently renderable.
struct WaveformLevel {
    std::vector<float> mins;
    std::vector<float> maxs;
    uint32_t samples_per_bucket = 0;
};

/// A waveform pyramid over the canonical mono mixdown. Levels are ordered
/// finest to coarsest.
struct Waveform {
    uint32_t canonical_sample_rate = 0;
    std::vector<WaveformLevel> levels;
};

/// Builds a waveform pyramid by streaming the source to the canonical
/// 48 kHz mono float mixdown; the original file is never modified.
///
/// `max_levels` bounds the pyramid height (at least 1). The base level uses
/// 480 samples (10 ms at the canonical rate).
///
/// @throws std::runtime_error when the source cannot be decoded.
Waveform build_waveform(const std::string& path, uint32_t max_levels);

} // namespace echo::audio
