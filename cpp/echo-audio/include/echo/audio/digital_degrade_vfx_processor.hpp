#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Bounded, deterministic digital-resolution degradation.
///
/// Bitcrusher and sample-rate reduction remain separate authored characters;
/// LoFi explicitly composes both stages. The stereo sample-and-hold clock is
/// shared across channels. Construction performs the only allocation, while
/// `update()`, `process_interleaved()`, and `reset()` allocate nothing.
class DigitalDegradeVfxProcessor {
  public:
    DigitalDegradeVfxProcessor(
        DigitalDegradeVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~DigitalDegradeVfxProcessor();

    DigitalDegradeVfxProcessor(const DigitalDegradeVfxProcessor&) = delete;
    DigitalDegradeVfxProcessor& operator=(const DigitalDegradeVfxProcessor&) = delete;

    void update(DigitalDegradeVfxAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
