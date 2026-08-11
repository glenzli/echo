#include "echo/audio/effect_mask_plan.hpp"

#include <algorithm>
#include <limits>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::size_t kMaximumMasks = 64;
constexpr std::uint64_t kMaximumFeatherMillis = 100;

std::uint64_t to_frames(std::uint64_t millis, std::uint32_t sample_rate) {
    if (sample_rate == 0 || millis > std::numeric_limits<std::uint64_t>::max() / sample_rate) {
        throw std::invalid_argument("effect mask time exceeds the supported range");
    }
    return millis * sample_rate / 1000U;
}

float smoothstep(float value) {
    const float bounded = std::clamp(value, 0.0F, 1.0F);
    return bounded * bounded * (3.0F - 2.0F * bounded);
}

} // namespace

EffectMaskPlan::EffectMaskPlan(
    const PlaybackAdjustment& authored,
    const PreparedAdjustment& prepared,
    std::uint32_t sample_rate
) {
    if (authored.effect_masks.size() > kMaximumMasks) {
        throw std::invalid_argument("effect mask count exceeds 64");
    }
    std::array<bool, kEffectNodeCount> active{};
    const auto chain = prepared.effect_chain();
    for (std::size_t index = 0; index < prepared.effect_chain_count(); ++index) {
        active[static_cast<std::size_t>(chain[index])] = true;
    }
    for (const EffectMask& mask : authored.effect_masks) {
        if (mask.start_millis >= mask.end_millis || mask.start_millis < prepared.trim_start_millis()
            || mask.end_millis > prepared.trim_end_millis()
            || mask.feather_millis > kMaximumFeatherMillis || mask.nodes.empty()) {
            throw std::invalid_argument("effect mask is outside the supported source range");
        }
        std::array<bool, kEffectNodeCount> seen{};
        for (const EffectNodeKind node : mask.nodes) {
            const std::size_t value = static_cast<std::size_t>(node);
            if (value >= kEffectNodeCount || seen[value] || !active[value]
                || node == EffectNodeKind::Master || node == EffectNodeKind::DeClick) {
                throw std::invalid_argument(
                    "effect mask nodes must be unique active maskable inserts"
                );
            }
            seen[value] = true;
            intervals_[value].push_back({
                .start_frame = to_frames(mask.start_millis, sample_rate),
                .end_frame = to_frames(mask.end_millis, sample_rate),
                .feather_frames = to_frames(mask.feather_millis, sample_rate),
            });
        }
    }
    for (auto& intervals : intervals_) {
        std::sort(
            intervals.begin(),
            intervals.end(),
            [](const Interval& left, const Interval& right) {
                return left.start_frame < right.start_frame;
            }
        );
    }
}

bool EffectMaskPlan::is_locally_masked(EffectNodeKind node) const {
    const std::size_t value = static_cast<std::size_t>(node);
    return value < intervals_.size() && !intervals_[value].empty();
}

float EffectMaskPlan::mix_at(EffectNodeKind node, std::uint64_t source_frame) const {
    const std::size_t value = static_cast<std::size_t>(node);
    if (value >= intervals_.size()) {
        return 0.0F;
    }
    const auto& intervals = intervals_[value];
    if (intervals.empty()) {
        return 1.0F;
    }
    if (source_frame == kNoSourceFrame) {
        return 0.0F;
    }
    float mix = 0.0F;
    for (const Interval& interval : intervals) {
        float candidate = 0.0F;
        if (source_frame >= interval.start_frame && source_frame < interval.end_frame) {
            if (interval.feather_frames == 0) {
                candidate = 1.0F;
            } else {
                const float fade_in = smoothstep(
                    static_cast<float>(source_frame - interval.start_frame)
                    / static_cast<float>(interval.feather_frames)
                );
                const float fade_out = smoothstep(
                    static_cast<float>(interval.end_frame - source_frame)
                    / static_cast<float>(interval.feather_frames)
                );
                candidate = std::min(fade_in, fade_out);
            }
        }
        mix = std::max(mix, candidate);
        if (mix >= 1.0F) {
            return 1.0F;
        }
    }
    return mix;
}

} // namespace echo::audio
