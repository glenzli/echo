#pragma once

#include <cstddef>
#include <cstdint>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Stereo-linked final-output limiter with bounded sub-block look-ahead.
///
/// Four phase points from a cubic reconstruction estimate inter-sample peaks.
/// The producer scans 64-frame sub-blocks before applying one linked gain
/// envelope, so the realtime callback remains a ring-buffer copy only.
class OutputLimiter {
  public:
    OutputLimiter(LimiterAdjustment adjustment, std::uint32_t sample_rate);

    void update(LimiterAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] float gain_reduction_decibels() const;

  private:
    [[nodiscard]] static float estimate_true_peak(
        const float* samples,
        std::size_t first_frame,
        std::size_t end_frame,
        std::size_t total_frames,
        std::size_t channel_count
    );

    std::uint32_t sample_rate_ = 0;
    bool enabled_ = false;
    float ceiling_amplitude_ = 1.0F;
    float release_coefficient_ = 0.0F;
    float gain_ = 1.0F;
};

} // namespace echo::audio
