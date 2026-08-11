#pragma once

#include "echo/audio/adjustment.hpp"

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// High-frequency split de-esser with a stereo-linked sidechain envelope.
class DeEsser {
  public:
    DeEsser(DeEsserAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);

    void update(DeEsserAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] float attenuation_decibels() const;

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    DeEsserAdjustment target_;
    std::vector<float> lowpass_state_;
    std::vector<float> high_components_;
    float frequency_hertz_ = 6500.0F;
    float threshold_centibels_ = -2400.0F;
    float reduction_centibels_ = 0.0F;
    float envelope_ = 0.0F;
    float gain_ = 1.0F;
};

} // namespace echo::audio
