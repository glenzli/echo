#pragma once

#include "echo/audio/adjustment.hpp"
#include "echo/audio/source_edit_plan.hpp"

#include <array>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Prepared original-time activation for insert effects.
///
/// A node named by at least one mask is wet only in the union of its masks.
/// Nodes absent from every mask remain global. Gap frames have no original
/// anchor, so local nodes remain dry while global nodes continue advancing.
class EffectMaskPlan {
  public:
    EffectMaskPlan(
        const PlaybackAdjustment& authored,
        const PreparedAdjustment& prepared,
        std::uint32_t sample_rate
    );

    [[nodiscard]] bool is_locally_masked(EffectNodeKind node) const;
    [[nodiscard]] float mix_at(EffectNodeKind node, std::uint64_t source_frame) const;

  private:
    struct Interval {
        std::uint64_t start_frame = 0;
        std::uint64_t end_frame = 0;
        std::uint64_t feather_frames = 0;
    };
    std::array<std::vector<Interval>, kEffectNodeCount> intervals_;
};

} // namespace echo::audio
