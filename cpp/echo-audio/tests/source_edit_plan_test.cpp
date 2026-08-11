#include "echo/audio/source_edit_plan.hpp"

#include <cassert>
#include <cmath>
#include <stdexcept>

namespace {

constexpr std::uint32_t kSampleRate = 48'000;

echo::audio::PlaybackAdjustment edited() {
    echo::audio::PlaybackAdjustment adjustment;
    adjustment.trim_end_millis = 1000;
    adjustment.edit_segments = {
        {
            .source_start_millis = 0,
            .source_end_millis = 250,
            .state = echo::audio::EditSegmentState::Audible,
            .gain_centibels = -600,
            .fade_in_millis = 50,
            .fade_out_millis = 50,
            .fade_in_curve = echo::audio::FadeCurve::Linear,
            .fade_out_curve = echo::audio::FadeCurve::Smooth,
        },
        {
            .source_start_millis = 250,
            .source_end_millis = 500,
            .state = echo::audio::EditSegmentState::Hidden,
            .gap_after_millis = 100,
        },
        {
            .source_start_millis = 500,
            .source_end_millis = 750,
            .state = echo::audio::EditSegmentState::Muted,
        },
        {
            .source_start_millis = 750,
            .source_end_millis = 1000,
            .state = echo::audio::EditSegmentState::Audible,
        },
    };
    return adjustment;
}

} // namespace

int main() {
    {
        echo::audio::PlaybackAdjustment legacy;
        legacy.trim_start_millis = 100;
        legacy.trim_end_millis = 600;
        const echo::audio::SourceEditPlan plan(legacy, 1000, kSampleRate);
        assert(plan.start_frame() == 4'800);
        assert(plan.end_frame() == 28'800);
        assert(plan.output_frame_count() == 24'000);
        assert(plan.output_duration_millis() == 500);
        assert(plan.frame_at(4'800).emitted);
    }

    {
        const echo::audio::SourceEditPlan plan(edited(), 1000, kSampleRate);
        assert(plan.output_frame_count() == 40'800);
        const auto faded = plan.frame_at(1'200);
        assert(faded.emitted && !faded.muted);
        assert(faded.amplitude > 0.20F && faded.amplitude < 0.30F);
        assert(!plan.frame_at(12'000).emitted);
        const auto hidden_end = plan.frame_at(23'999);
        assert(!hidden_end.emitted && hidden_end.gap_after_frames == 4'800);
        const auto muted = plan.frame_at(24'000);
        assert(muted.emitted && muted.muted && muted.amplitude == 0.0F);
        // Seeking into a hidden source interval advances to the next
        // source-backed output boundary.
        assert(plan.clamp_source_seek_millis(300) == 500);
        // The automatic join edge is quiet at the first frame following a
        // collapsed hidden interval and reaches unity after five ms.
        assert(plan.frame_at(36'000).amplitude == 0.0F);
        assert(plan.frame_at(36'240).amplitude > 0.99F);
    }

    {
        echo::audio::PlaybackAdjustment all_hidden;
        all_hidden.trim_end_millis = 10;
        all_hidden.edit_segments = {{
            .source_start_millis = 0,
            .source_end_millis = 10,
            .state = echo::audio::EditSegmentState::Hidden,
            .gap_after_millis = 20,
        }};
        const echo::audio::SourceEditPlan plan(all_hidden, 10, kSampleRate);
        assert(plan.output_frame_count() == 960);
    }

    {
        echo::audio::PlaybackAdjustment short_segments;
        short_segments.trim_end_millis = 2;
        short_segments.edit_segments = {
            {
                .source_start_millis = 0,
                .source_end_millis = 1,
                .state = echo::audio::EditSegmentState::Audible,
            },
            {
                .source_start_millis = 1,
                .source_end_millis = 2,
                .state = echo::audio::EditSegmentState::Muted,
            },
        };
        const echo::audio::SourceEditPlan plan(short_segments, 2, kSampleRate);
        assert(plan.output_frame_count() == 96);
        assert(std::isfinite(plan.frame_at(47).amplitude));
        assert(plan.frame_at(48).muted);
    }

    {
        auto invalid = edited();
        invalid.edit_segments[1].source_start_millis = 251;
        bool rejected = false;
        try {
            [[maybe_unused]] const echo::audio::SourceEditPlan plan(invalid, 1000, kSampleRate);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }
}
