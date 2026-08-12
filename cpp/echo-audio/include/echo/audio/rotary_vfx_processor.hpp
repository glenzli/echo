#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <memory>

namespace echo::audio {

/// Deterministic dual-rotor amplitude and Doppler motion.
///
/// A fixed crossover feeds independent drum and horn rotors with different
/// speed inertia. Short fractional delays are authored wet content, like a
/// chorus delay, rather than infrastructure latency. The dry path therefore
/// remains sample aligned and `latency_frames()` is zero. Construction performs
/// the only heap allocation; `update()`, `process_interleaved()`, and `reset()`
/// allocate nothing and take no locks.
class RotaryVfxProcessor {
  public:
    RotaryVfxProcessor(
        RotaryVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~RotaryVfxProcessor();

    RotaryVfxProcessor(const RotaryVfxProcessor&) = delete;
    RotaryVfxProcessor& operator=(const RotaryVfxProcessor&) = delete;

    void update(RotaryVfxAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] RotaryVfxAdjustment adjustment() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
