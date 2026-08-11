#pragma once

#include "echo/audio/adjustment.hpp"

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

inline constexpr std::uint64_t kNoSourceFrame = ~std::uint64_t{0};

struct SourceEditFrame {
    bool emitted = false;
    bool muted = false;
    float amplitude = 0.0F;
    std::uint64_t gap_after_frames = 0;
};

/// Immutable source-time edit decision prepared before decoding begins.
///
/// Authored segments stay in original coordinates. Hidden intervals collapse,
/// muted intervals keep duration, and gaps add output-only frames. The plan is
/// allocation-free after construction and is shared by playback, analysis,
/// and offline render through PlaybackSession.
class SourceEditPlan {
  public:
    SourceEditPlan(
        const PlaybackAdjustment& authored,
        std::uint64_t source_duration_millis,
        std::uint32_t sample_rate
    );

    [[nodiscard]] std::uint64_t start_frame() const;
    [[nodiscard]] std::uint64_t end_frame() const;
    [[nodiscard]] std::uint64_t output_frame_count() const;
    [[nodiscard]] std::uint64_t output_duration_millis() const;
    [[nodiscard]] std::uint64_t clamp_source_seek_millis(std::uint64_t millis) const;
    [[nodiscard]] SourceEditFrame frame_at(std::uint64_t source_frame) const;

  private:
    struct Segment {
        std::uint64_t start_frame = 0;
        std::uint64_t end_frame = 0;
        EditSegmentState state = EditSegmentState::Audible;
        float gain = 1.0F;
        std::uint64_t fade_in_frames = 0;
        std::uint64_t fade_out_frames = 0;
        FadeCurve fade_in_curve = FadeCurve::Linear;
        FadeCurve fade_out_curve = FadeCurve::Linear;
        std::uint64_t gap_after_frames = 0;
        std::uint64_t join_fade_in_frames = 0;
        std::uint64_t join_fade_out_frames = 0;
    };
    std::vector<Segment> segments_;
    std::uint64_t start_frame_ = 0;
    std::uint64_t end_frame_ = 0;
    std::uint64_t output_frame_count_ = 0;
    std::uint32_t sample_rate_ = 0;
};

} // namespace echo::audio
