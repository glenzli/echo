#pragma once

#include <cstddef>
#include <cstdint>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Stereo-linked feed-forward soft-knee compressor.
///
/// The processor owns its detector envelope across decode chunks. Parameter
/// changes update the target gain without resetting that envelope, preventing
/// clicks during live preview. It allocates nothing while processing.
class DynamicsProcessor {
  public:
    DynamicsProcessor(CompressorAdjustment adjustment, std::uint32_t sample_rate);

    void update(CompressorAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] CompressorAdjustment adjustment() const;
    [[nodiscard]] float current_gain() const;

  private:
    void validate(CompressorAdjustment adjustment) const;
    void refresh_coefficients();
    [[nodiscard]] float desired_gain(float peak) const;

    CompressorAdjustment adjustment_;
    std::uint32_t sample_rate_ = 0;
    float attack_coefficient_ = 1.0F;
    float release_coefficient_ = 1.0F;
    float gain_ = 1.0F;
};

} // namespace echo::audio
