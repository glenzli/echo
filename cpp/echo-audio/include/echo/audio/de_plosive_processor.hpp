#pragma once

#include "echo/audio/adjustment.hpp"

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Zero-latency, stereo-linked suppression of short low-frequency plosive bursts.
///
/// The processor continuously separates a bounded low band, compares its fast
/// envelope with a slower local baseline, and attenuates only that band during
/// low-dominant onsets. All scratch storage is prepared in the constructor;
/// authored changes and bypass are sample-smoothed without allocating.
class DePlosiveProcessor {
  public:
    DePlosiveProcessor(
        DePlosiveAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    void update(DePlosiveAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }
    [[nodiscard]] DePlosiveAdjustment adjustment() const;
    [[nodiscard]] float attenuation_decibels() const;

  private:
    static void
    validate(DePlosiveAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);
    [[nodiscard]] float coefficient(float milliseconds) const;
    [[nodiscard]] float lowpass_alpha(std::uint16_t frequency_hertz) const;
    void refresh_targets();

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    DePlosiveAdjustment target_;
    std::vector<float> lowpass_state_;
    std::vector<float> low_components_;
    float parameter_step_ = 0.0F;
    float lowpass_alpha_ = 0.0F;
    float target_lowpass_alpha_ = 0.0F;
    float sensitivity_ = 0.0F;
    float target_sensitivity_ = 0.0F;
    float minimum_gain_ = 1.0F;
    float target_minimum_gain_ = 1.0F;
    float release_step_ = 0.0F;
    float target_release_step_ = 0.0F;
    float fast_low_envelope_ = 0.0F;
    float slow_low_envelope_ = 0.0F;
    float broadband_envelope_ = 0.0F;
    float gain_ = 1.0F;
};

} // namespace echo::audio
