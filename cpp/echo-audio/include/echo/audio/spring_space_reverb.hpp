#pragma once

#include "echo/audio/adjustment.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Clean-room three-path dispersive spring space for 48 kHz mono/stereo PCM.
///
/// The implementation prepares every delay buffer in the constructor. Valid
/// `update()`, `process_interleaved()`, and `reset()` calls neither allocate nor
/// lock. Structural edits crossfade complete preallocated states, while Mix
/// and Enabled use a shorter sample ramp. The dry path has no lookahead;
/// propagation and authored pre-delay are wet-effect content rather than
/// infrastructure latency.
class SpringSpaceReverb {
  public:
    SpringSpaceReverb(
        ReverbAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~SpringSpaceReverb();

    SpringSpaceReverb(const SpringSpaceReverb&) = delete;
    SpringSpaceReverb& operator=(const SpringSpaceReverb&) = delete;

    static void
    validate(ReverbAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);

    void update(ReverbAdjustment adjustment);
    [[nodiscard]] std::array<float, 2> process_frame(float left, float right);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] std::size_t latency_frames() const noexcept;
    [[nodiscard]] bool is_bypassed() const noexcept;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
