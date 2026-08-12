#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <memory>

namespace echo::audio {

/// Deterministic, input-driven saturation with fixed-latency antialiasing.
///
/// The three characters are explicit listening roles, not physical-model
/// claims. A two-times, linear-phase FIR path bounds fold-back from the
/// memoryless nonlinearities. The node always retains its 32-frame delay while
/// present, including bypass and zero-mix operation, so later graph wiring can
/// compensate it without a timeline jump. Construction performs the only heap
/// allocation; `update()`, `process_interleaved()`, and `reset()` allocate
/// nothing.
class DriveVfxProcessor {
  public:
    DriveVfxProcessor(
        DriveVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~DriveVfxProcessor();

    DriveVfxProcessor(const DriveVfxProcessor&) = delete;
    DriveVfxProcessor& operator=(const DriveVfxProcessor&) = delete;

    void update(DriveVfxAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] DriveVfxAdjustment adjustment() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 32;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
