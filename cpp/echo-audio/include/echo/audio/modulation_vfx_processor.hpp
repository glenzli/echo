#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Bounded Chorus, Flanger, Phaser, and Tremolo processing.
///
/// Each character retains a typed algorithm and authored parameter set. All
/// delay storage is prepared in the constructor; updates and processing are
/// allocation-free. Modulated delay is authored wet content, not chain
/// latency, so the processor always reports zero compensation frames.
class ModulationVfxProcessor {
  public:
    ModulationVfxProcessor(
        ModulationVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~ModulationVfxProcessor();

    ModulationVfxProcessor(const ModulationVfxProcessor&) = delete;
    ModulationVfxProcessor& operator=(const ModulationVfxProcessor&) = delete;

    void update(ModulationVfxAdjustment adjustment);
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
