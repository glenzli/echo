#pragma once

#include "echo/audio/adjustment.hpp"

#include <array>
#include <cstddef>
#include <cstdint>

namespace echo::audio {

/// Zero-latency stereo fault repair compiled to one smoothed channel matrix.
///
/// Construction owns all state. `process_interleaved()` performs no
/// allocation, keeps its transition continuous across blocks, and supports
/// mono input as a bounded polarity-only fallback.
class ChannelRepairProcessor {
  public:
    ChannelRepairProcessor(
        ChannelRepairAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    void update(ChannelRepairAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

  private:
    using Matrix = std::array<float, 4>;

    [[nodiscard]] Matrix matrix_for(ChannelRepairAdjustment adjustment) const;
    void validate(ChannelRepairAdjustment adjustment) const;

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t transition_length_frames_ = 0;
    std::size_t transition_frames_remaining_ = 0;
    Matrix current_{{1.0F, 0.0F, 0.0F, 1.0F}};
    Matrix target_{{1.0F, 0.0F, 0.0F, 1.0F}};
    Matrix step_{};
};

} // namespace echo::audio
