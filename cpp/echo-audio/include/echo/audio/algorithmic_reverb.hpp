#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Portable stereo algorithmic space effect with click-free authored updates.
///
/// Room preserves Echo's original four-line engine. Hall and Plate dispatch to
/// the separately owned diffuse FDN; Spring dispatches to a dispersive waveguide
/// owner. Parameter changes crossfade complete engines so topology, Size, or
/// Pre-delay edits cannot expose discontinuities.
class AlgorithmicReverb {
  public:
    AlgorithmicReverb(
        ReverbAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~AlgorithmicReverb();

    AlgorithmicReverb(const AlgorithmicReverb&) = delete;
    AlgorithmicReverb& operator=(const AlgorithmicReverb&) = delete;

    void update(ReverbAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();
    [[nodiscard]] bool is_bypassed() const;

  private:
    struct Engine;

    static void
    validate(ReverbAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);
    static bool same(ReverbAdjustment left, ReverbAdjustment right);
    void begin_transition(ReverbAdjustment adjustment);

    std::uint32_t sample_rate_;
    std::size_t channel_count_;
    std::unique_ptr<Engine> active_;
    std::unique_ptr<Engine> next_;
    std::optional<ReverbAdjustment> pending_;
    float ducking_envelope_ = 0.0F;
    std::size_t transition_frame_ = 0;
    std::size_t transition_frames_ = 1;
};

} // namespace echo::audio
