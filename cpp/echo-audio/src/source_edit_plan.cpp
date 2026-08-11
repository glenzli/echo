#include "echo/audio/source_edit_plan.hpp"

#include <algorithm>
#include <cmath>
#include <limits>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::size_t kMaximumSegments = 128;
constexpr std::uint64_t kMaximumGapMillis = 60ULL * 60ULL * 1000ULL;
constexpr std::uint64_t kJoinSmoothingMillis = 5;
constexpr std::int16_t kMinimumGainCentibels = -2400;
constexpr std::int16_t kMaximumGainCentibels = 1200;
constexpr float kHalfPi = 1.5707963267948966F;

std::uint64_t to_frames(std::uint64_t millis, std::uint32_t sample_rate) {
    if (sample_rate == 0 || millis > std::numeric_limits<std::uint64_t>::max() / sample_rate) {
        throw std::invalid_argument("source edit time exceeds the supported range");
    }
    return millis * sample_rate / 1000U;
}

bool valid_curve(FadeCurve curve) {
    return curve == FadeCurve::Linear || curve == FadeCurve::Smooth
           || curve == FadeCurve::EqualPower;
}

bool valid_state(EditSegmentState state) {
    return state == EditSegmentState::Audible || state == EditSegmentState::Muted
           || state == EditSegmentState::Hidden;
}

float curve_at(float progress, FadeCurve curve) {
    const float bounded = std::clamp(progress, 0.0F, 1.0F);
    switch (curve) {
    case FadeCurve::Linear:
        return bounded;
    case FadeCurve::Smooth:
        return bounded * bounded * (3.0F - 2.0F * bounded);
    case FadeCurve::EqualPower:
        return std::sin(bounded * kHalfPi);
    }
    return bounded;
}

} // namespace

SourceEditPlan::SourceEditPlan(
    const PlaybackAdjustment& authored,
    std::uint64_t source_duration_millis,
    std::uint32_t sample_rate
) : sample_rate_(sample_rate) {
    if (source_duration_millis == 0 || sample_rate == 0) {
        throw std::invalid_argument("source edit requires a known source layout");
    }
    const std::uint64_t trim_end =
        authored.trim_end_millis == 0 ? source_duration_millis : authored.trim_end_millis;
    if (authored.trim_start_millis >= trim_end || trim_end > source_duration_millis) {
        throw std::invalid_argument("source edit trim range is outside the source");
    }

    std::vector<EditSegment> authored_segments = authored.edit_segments;
    if (authored_segments.empty()) {
        authored_segments.push_back({
            .source_start_millis = authored.trim_start_millis,
            .source_end_millis = trim_end,
            .state = EditSegmentState::Audible,
        });
    }
    if (authored_segments.empty() || authored_segments.size() > kMaximumSegments) {
        throw std::invalid_argument("source edit requires between 1 and 128 segments");
    }

    segments_.reserve(authored_segments.size());
    std::uint64_t expected_start = authored.trim_start_millis;
    for (const EditSegment& segment : authored_segments) {
        if (segment.source_start_millis != expected_start
            || segment.source_start_millis >= segment.source_end_millis
            || segment.source_end_millis > trim_end) {
            throw std::invalid_argument("source edit segments must continuously cover the trim");
        }
        const std::uint64_t duration = segment.source_end_millis - segment.source_start_millis;
        if (!valid_state(segment.state) || !valid_curve(segment.fade_in_curve)
            || !valid_curve(segment.fade_out_curve)
            || segment.fade_in_millis + segment.fade_out_millis > duration
            || segment.gain_centibels < kMinimumGainCentibels
            || segment.gain_centibels > kMaximumGainCentibels
            || segment.gap_after_millis > kMaximumGapMillis) {
            throw std::invalid_argument("source edit segment is outside the supported range");
        }
        Segment prepared{
            .start_frame = to_frames(segment.source_start_millis, sample_rate),
            .end_frame = to_frames(segment.source_end_millis, sample_rate),
            .state = segment.state,
            .gain = std::pow(10.0F, static_cast<float>(segment.gain_centibels) / 2000.0F),
            .fade_in_frames = to_frames(segment.fade_in_millis, sample_rate),
            .fade_out_frames = to_frames(segment.fade_out_millis, sample_rate),
            .fade_in_curve = segment.fade_in_curve,
            .fade_out_curve = segment.fade_out_curve,
            .gap_after_frames = to_frames(segment.gap_after_millis, sample_rate),
        };
        const std::uint64_t source_frames = prepared.end_frame - prepared.start_frame;
        const std::uint64_t emitted_frames =
            prepared.state == EditSegmentState::Hidden ? 0 : source_frames;
        if (emitted_frames > std::numeric_limits<std::uint64_t>::max() - output_frame_count_
            || prepared.gap_after_frames > std::numeric_limits<std::uint64_t>::max()
                                               - output_frame_count_ - emitted_frames) {
            throw std::invalid_argument("source edit output duration exceeds the supported range");
        }
        output_frame_count_ += emitted_frames + prepared.gap_after_frames;
        segments_.push_back(prepared);
        expected_start = segment.source_end_millis;
    }
    if (expected_start != trim_end) {
        throw std::invalid_argument("source edit segments must continuously cover the trim");
    }
    if (output_frame_count_ == 0) {
        throw std::invalid_argument("source edit must produce audible time or an authored gap");
    }

    start_frame_ = segments_.front().start_frame;
    end_frame_ = segments_.back().end_frame;

    const std::uint64_t join_frames = to_frames(kJoinSmoothingMillis, sample_rate);
    for (std::size_t index = 0; index < segments_.size(); ++index) {
        Segment& current = segments_[index];
        if (current.state != EditSegmentState::Audible) {
            continue;
        }
        const std::uint64_t duration = current.end_frame - current.start_frame;
        bool discontinuity_before = false;
        bool discontinuity_after = false;
        if (index > 0) {
            const Segment& previous = segments_[index - 1];
            discontinuity_before =
                previous.state != EditSegmentState::Audible || previous.gap_after_frames > 0;
        }
        if (index + 1 < segments_.size()) {
            discontinuity_after = segments_[index + 1].state != EditSegmentState::Audible
                                  || current.gap_after_frames > 0;
        }
        current.join_fade_in_frames = discontinuity_before ? std::min(join_frames, duration) : 0;
        current.join_fade_out_frames = discontinuity_after ? std::min(join_frames, duration) : 0;
    }
}

std::uint64_t SourceEditPlan::start_frame() const {
    return start_frame_;
}

std::uint64_t SourceEditPlan::end_frame() const {
    return end_frame_;
}

std::uint64_t SourceEditPlan::output_frame_count() const {
    return output_frame_count_;
}

std::uint64_t SourceEditPlan::output_duration_millis() const {
    return output_frame_count_ * 1000U / sample_rate_;
}

std::uint64_t SourceEditPlan::clamp_source_seek_millis(std::uint64_t millis) const {
    const std::uint64_t frame =
        std::clamp(to_frames(millis, sample_rate_), start_frame_, end_frame_ - 1);
    const auto found = std::upper_bound(
        segments_.begin(),
        segments_.end(),
        frame,
        [](std::uint64_t value, const Segment& segment) { return value < segment.end_frame; }
    );
    if (found == segments_.end() || found->state != EditSegmentState::Hidden) {
        return frame * 1000U / sample_rate_;
    }
    for (auto candidate = found + 1; candidate != segments_.end(); ++candidate) {
        if (candidate->state != EditSegmentState::Hidden) {
            return candidate->start_frame * 1000U / sample_rate_;
        }
    }
    // An all-hidden tail can still own a gap. Keep the seek source-anchored
    // at the start of that hidden run so decoding can materialize the gap.
    return found->start_frame * 1000U / sample_rate_;
}

SourceEditFrame SourceEditPlan::frame_at(std::uint64_t source_frame) const {
    const auto found = std::upper_bound(
        segments_.begin(),
        segments_.end(),
        source_frame,
        [](std::uint64_t frame, const Segment& segment) { return frame < segment.end_frame; }
    );
    if (found == segments_.end() || source_frame < found->start_frame) {
        return {};
    }
    const Segment& segment = *found;
    SourceEditFrame result{
        .emitted = segment.state != EditSegmentState::Hidden,
        .muted = segment.state == EditSegmentState::Muted,
        .amplitude = segment.state == EditSegmentState::Audible ? segment.gain : 0.0F,
        .gap_after_frames = source_frame + 1 == segment.end_frame ? segment.gap_after_frames : 0,
    };
    if (!result.emitted || result.muted) {
        return result;
    }
    const std::uint64_t offset = source_frame - segment.start_frame;
    const std::uint64_t remaining = segment.end_frame - source_frame;
    if (segment.fade_in_frames > 0 && offset < segment.fade_in_frames) {
        result.amplitude *= curve_at(
            static_cast<float>(offset) / static_cast<float>(segment.fade_in_frames),
            segment.fade_in_curve
        );
    }
    if (segment.fade_out_frames > 0 && remaining <= segment.fade_out_frames) {
        result.amplitude *= curve_at(
            static_cast<float>(remaining) / static_cast<float>(segment.fade_out_frames),
            segment.fade_out_curve
        );
    }
    if (segment.join_fade_in_frames > 0 && offset < segment.join_fade_in_frames) {
        result.amplitude *=
            static_cast<float>(offset) / static_cast<float>(segment.join_fade_in_frames);
    }
    if (segment.join_fade_out_frames > 0 && remaining <= segment.join_fade_out_frames) {
        result.amplitude *=
            static_cast<float>(remaining) / static_cast<float>(segment.join_fade_out_frames);
    }
    return result;
}

} // namespace echo::audio
