#pragma once

#include <cstddef>
#include <cstdint>

namespace echo::audio {

/// Final preview-output safety stage.
///
/// A short attack/release envelope reduces block peaks before a differentiable
/// ceiling catches any residual overshoot. This is not an authored adjustment
/// and never enters the AdjustmentGraph; it prevents the device boundary from
/// turning valid restoration boosts into hard-clipped preview audio.
class OutputGuard {
  public:
    explicit OutputGuard(std::uint32_t sample_rate);

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

  private:
    [[nodiscard]] static float soft_ceiling(float sample);

    float attack_coefficient_ = 0.0F;
    float release_coefficient_ = 0.0F;
    float gain_ = 1.0F;
    bool initialized_ = false;
};

} // namespace echo::audio
