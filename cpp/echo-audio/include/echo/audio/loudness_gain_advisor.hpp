#pragma once

#include <cstdint>

namespace echo::audio {

struct LoudnessGainRequest {
    float integrated_lufs = -70.0F;
    float true_peak_dbtp = -70.0F;
    float target_lufs = -16.0F;
    float true_peak_ceiling_dbtp = -1.0F;
    std::int16_t current_gain_centibels = 0;
};

struct LoudnessGainAdvice {
    bool available = false;
    bool peak_constrained = false;
    bool gain_range_constrained = false;
    bool target_reached = false;
    std::int16_t gain_delta_centibels = 0;
    std::int16_t resulting_gain_centibels = 0;
    float estimated_integrated_lufs = -70.0F;
    float estimated_true_peak_dbtp = -70.0F;
};

/// Produces a transparent clip-gain recommendation from a whole-preview scan.
///
/// The recommendation never asks the downstream limiter to create loudness:
/// positive gain is bounded by the measured true-peak headroom. It is also
/// quantized to the editor's 0.1 dB authored step without crossing that bound.
class LoudnessGainAdvisor {
  public:
    [[nodiscard]] static LoudnessGainAdvice advise(LoudnessGainRequest request);
};

} // namespace echo::audio
