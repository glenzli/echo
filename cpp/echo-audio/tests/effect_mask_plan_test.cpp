#include "echo/audio/effect_mask_plan.hpp"

#include <cassert>
#include <cmath>
#include <cstdint>
#include <stdexcept>

namespace {

constexpr std::uint32_t kSampleRate = 48'000;

echo::audio::PlaybackAdjustment masked() {
    echo::audio::PlaybackAdjustment adjustment;
    adjustment.trim_end_millis = 1000;
    adjustment.effect_masks = {
        {
            .start_millis = 100,
            .end_millis = 300,
            .feather_millis = 50,
            .nodes = {echo::audio::EffectNodeKind::Equalizer},
        },
        {
            .start_millis = 250,
            .end_millis = 500,
            .nodes = {
                echo::audio::EffectNodeKind::Equalizer,
                echo::audio::EffectNodeKind::Dynamics,
            },
        },
    };
    return adjustment;
}

} // namespace

int main() {
    {
        const auto adjustment = masked();
        const echo::audio::PreparedAdjustment prepared(adjustment, 1000, kSampleRate);
        const echo::audio::EffectMaskPlan plan(adjustment, prepared, kSampleRate);
        assert(plan.is_locally_masked(echo::audio::EffectNodeKind::Equalizer));
        assert(plan.is_locally_masked(echo::audio::EffectNodeKind::Dynamics));
        assert(!plan.is_locally_masked(echo::audio::EffectNodeKind::Restoration));
        assert(plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 0) == 0.0F);
        assert(plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 4'799) == 0.0F);
        assert(plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 4'800) == 0.0F);
        assert(plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 7'200) == 1.0F);
        assert(plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 20'000) == 1.0F);
        assert(plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 28'800) == 0.0F);
        const float feather = plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 6'000);
        assert(feather > 0.45F && feather < 0.55F);
        // A standalone feathered interval never leaks wet signal outside its
        // authored bounds. The overlapping second interval owns later union.
        auto single = adjustment;
        single.effect_masks.resize(1);
        const echo::audio::PreparedAdjustment single_prepared(single, 1000, kSampleRate);
        const echo::audio::EffectMaskPlan single_plan(single, single_prepared, kSampleRate);
        assert(single_plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 4'799) == 0.0F);
        assert(single_plan.mix_at(echo::audio::EffectNodeKind::Equalizer, 14'400) == 0.0F);
        assert(
            plan.mix_at(echo::audio::EffectNodeKind::Equalizer, echo::audio::kNoSourceFrame) == 0.0F
        );
        assert(
            plan.mix_at(echo::audio::EffectNodeKind::Restoration, echo::audio::kNoSourceFrame)
            == 1.0F
        );
    }

    {
        auto invalid = masked();
        for (const auto node : {
                 echo::audio::EffectNodeKind::DeClick,
                 echo::audio::EffectNodeKind::TransformVfx,
             }) {
            invalid.effect_masks[0].nodes = {node};
            if (node == echo::audio::EffectNodeKind::TransformVfx) {
                invalid.effect_chain = {
                    echo::audio::EffectNodeKind::TransformVfx,
                    echo::audio::EffectNodeKind::Master,
                    echo::audio::EffectNodeKind::Restoration,
                    echo::audio::EffectNodeKind::Equalizer,
                    echo::audio::EffectNodeKind::Dynamics,
                    echo::audio::EffectNodeKind::Space,
                    echo::audio::EffectNodeKind::DeHum,
                    echo::audio::EffectNodeKind::DeClick,
                    echo::audio::EffectNodeKind::ChannelRepair,
                    echo::audio::EffectNodeKind::SceneVfx,
                    echo::audio::EffectNodeKind::DelayVfx,
                    echo::audio::EffectNodeKind::ModulationVfx,
                    echo::audio::EffectNodeKind::DigitalDegradeVfx,
                };
                invalid.effect_chain_count = 2;
            }
            const echo::audio::PreparedAdjustment prepared(invalid, 1000, kSampleRate);
            bool rejected = false;
            try {
                [[maybe_unused]] const echo::audio::EffectMaskPlan plan(
                    invalid,
                    prepared,
                    kSampleRate
                );
            } catch (const std::invalid_argument&) {
                rejected = true;
            }
            assert(rejected);
        }
    }
}
