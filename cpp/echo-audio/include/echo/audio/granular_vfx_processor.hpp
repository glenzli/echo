#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Deterministic, source-derived granular playback for canonical Echo PCM.
///
/// The processor retains two seconds of mono or stereo input and schedules a
/// fixed pool of sixteen Hann-windowed grains. Random choices are a pure
/// function of the authored seed, linear processor-input frame, and grain
/// ordinal. This timeline is deliberately not an Original-source coordinate:
/// edits can collapse source intervals and insert gaps.
/// Voices read captured history incrementally, so no grain is pre-rendered in
/// a single callback frame. The dry path is immediate, so capture history is
/// wet content rather than infrastructure latency. Construction performs all
/// allocation; `update()`, `process_interleaved()`, and `reset()` allocate
/// nothing and take no locks.
///
/// Bypass stops scheduling new grains but continues to capture input. A fresh
/// or reset processor fed only digital silence therefore produces no wet
/// signal, while a bypassed processor can resume from recently captured source.
class GranularVfxProcessor {
  public:
    GranularVfxProcessor(
        GranularVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~GranularVfxProcessor();

    GranularVfxProcessor(const GranularVfxProcessor&) = delete;
    GranularVfxProcessor& operator=(const GranularVfxProcessor&) = delete;

    void update(GranularVfxAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    /// Clears capture and grains and establishes the linear processor-input
    /// timeline used for deterministic scheduling. Playback seek intentionally
    /// passes zero and starts a fresh texture; it does not claim to reconstruct
    /// up to two seconds of pre-seek grain history.
    void reset(std::uint64_t timeline_frame = 0) noexcept;

    /// Reports authored dry intent only. The host must continue processing a
    /// bypassed instance so its bounded source-history capture keeps advancing.
    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] GranularVfxAdjustment adjustment() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
