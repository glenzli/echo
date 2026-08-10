#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

#include "echo/audio/k_weighting_filter.hpp"

namespace echo::audio {

/// One read-only snapshot of the prepared device signal.
struct LoudnessSnapshot {
    float momentary_lufs = -70.0F;
    float sample_peak_dbfs = -70.0F;
};

/// K-weighted 400 ms momentary loudness and decaying sample-peak meter.
///
/// The meter follows the BS.1770 K-weighting stages and evaluates the exact
/// rolling 400 ms energy window used by EBU momentary loudness. It allocates
/// its fixed window once and never mutates the audio it observes.
class LoudnessMeter {
  public:
    LoudnessMeter(std::uint32_t sample_rate, std::size_t channel_count);

    void reset();
    void
    process_interleaved(const float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] LoudnessSnapshot snapshot() const;

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    KWeightingFilter weighting_;
    std::vector<double> energy_window_;
    std::size_t energy_cursor_ = 0;
    std::size_t energy_count_ = 0;
    double energy_sum_ = 0.0;
    double peak_amplitude_ = 0.0;
    double peak_release_per_frame_ = 1.0;
};

} // namespace echo::audio
