#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Source-driven scene coloration for Echo's explicit Creative VFX domain.
///
/// All delay storage and filter banks are prepared at construction. Updates
/// and processing allocate nothing, parameter changes crossfade between two
/// prepared banks, and every character retains a direct frame-zero path so
/// the processor has no algorithmic latency.
class SceneVfxProcessor {
  public:
    SceneVfxProcessor(
        SceneVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~SceneVfxProcessor();

    SceneVfxProcessor(const SceneVfxProcessor&) = delete;
    SceneVfxProcessor& operator=(const SceneVfxProcessor&) = delete;

    static void
    validate(SceneVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);

    void update(SceneVfxAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] std::size_t latency_frames() const;
    [[nodiscard]] bool is_bypassed() const;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
