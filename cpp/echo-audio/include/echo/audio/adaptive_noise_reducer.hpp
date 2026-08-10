#pragma once

#include "echo/audio/adjustment.hpp"

#include <cstddef>
#include <cstdint>

namespace echo::audio {

/// Stereo-linked downward expander with an adaptive noise-floor estimate.
/// All state and parameter ramps live on the decode producer thread.
class AdaptiveNoiseReducer {
  public:
    AdaptiveNoiseReducer(NoiseReductionAdjustment adjustment, std::uint32_t sample_rate);

    void update(NoiseReductionAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] float attenuation_decibels() const;

  private:
    std::uint32_t sample_rate_ = 0;
    NoiseReductionAdjustment target_;
    float envelope_ = 0.0F;
    float noise_floor_ = 0.0025F;
    float gain_ = 1.0F;
    float reduction_decibels_ = 0.0F;
    float sensitivity_ = 0.5F;
};

} // namespace echo::audio
