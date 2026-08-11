#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Bounded source-anchored slapback/echo processing.
///
/// Storage for the maximum authored delay is prepared in the constructor.
/// `update()` and `process_interleaved()` allocate nothing. The delayed taps
/// are creative wet content rather than infrastructure latency, so this node
/// always reports zero frames of chain compensation.
class DelayVfxProcessor {
  public:
    DelayVfxProcessor(
        DelayVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~DelayVfxProcessor();

    DelayVfxProcessor(const DelayVfxProcessor&) = delete;
    DelayVfxProcessor& operator=(const DelayVfxProcessor&) = delete;

    void update(DelayVfxAdjustment adjustment);
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
