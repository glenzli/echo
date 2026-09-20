#pragma once
#include "echo/audio/assembly.hpp"
#include "echo/audio/offline_render.hpp"
#include <memory>

namespace echo::audio {
// Stateful producer-side mix execution, shared by device audition and export.
// Fixed block partitioning preserves limiter lookahead across both consumers.
// Returned samples remain valid until next()/seek(); never call from a device callback.
class AssemblyMixer {
  public:
    static constexpr std::size_t block_frames = 4096;
    explicit AssemblyMixer(AssemblyMixPlan plan);
    ~AssemblyMixer();
    AssemblyMixer(const AssemblyMixer&) = delete;
    AssemblyMixer& operator=(const AssemblyMixer&) = delete;
    std::span<const float> next(const OfflineRenderCallbacks& callbacks = {});
    // Milliseconds relative to the prepared preview window. Seeking resets DSP history.
    void seek(std::uint64_t millis);
    std::uint64_t frame_count() const;
    std::uint64_t position_frames() const;
    float limiter_reduction_decibels() const;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};
} // namespace echo::audio
