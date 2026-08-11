#include "echo/audio/effect_processing_chain.hpp"

#include <algorithm>
#include <array>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;
constexpr std::size_t kChannels = 2;

echo::audio::PlaybackAdjustment master_only() {
    echo::audio::PlaybackAdjustment adjustment;
    adjustment.trim_end_millis = 1000;
    adjustment.effect_chain = {
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
    };
    adjustment.effect_chain_count = 1;
    return adjustment;
}

echo::audio::PlaybackAdjustment bypassed_de_click() {
    auto adjustment = master_only();
    adjustment.effect_chain = {
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
    };
    adjustment.effect_chain_count = 3;
    return adjustment;
}

echo::audio::PlaybackAdjustment reordered_bypassed_de_click() {
    auto adjustment = bypassed_de_click();
    adjustment.effect_chain = {
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
    };
    return adjustment;
}

std::vector<float> fixture(std::size_t frame_count) {
    std::vector<float> samples(frame_count * kChannels);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float value = static_cast<float>(frame + 1) / static_cast<float>(frame_count + 1);
        samples[frame * kChannels] = value;
        samples[frame * kChannels + 1] = -0.5F * value;
    }
    return samples;
}

std::vector<float> process_in_chunks(
    echo::audio::EffectProcessingChain& chain,
    const std::vector<float>& input,
    std::size_t chunk_frames
) {
    const std::size_t input_frames = input.size() / kChannels;
    std::vector<float> output;
    std::vector<float> scratch(chunk_frames * kChannels);
    std::size_t consumed = 0;
    while (consumed < input_frames) {
        const std::size_t frames = std::min(chunk_frames, input_frames - consumed);
        std::copy_n(input.data() + consumed * kChannels, frames * kChannels, scratch.data());
        const std::size_t produced = chain.process_interleaved(scratch.data(), frames, kChannels);
        output.insert(output.end(), scratch.data(), scratch.data() + produced * kChannels);
        consumed += frames;
    }
    while (chain.pending_output_frames() > 0) {
        const std::size_t produced =
            chain.finish_interleaved(scratch.data(), chunk_frames, kChannels);
        assert(produced > 0);
        output.insert(output.end(), scratch.data(), scratch.data() + produced * kChannels);
    }
    return output;
}

void assert_near(const std::vector<float>& actual, const std::vector<float>& expected) {
    assert(actual.size() == expected.size());
    for (std::size_t index = 0; index < actual.size(); ++index) {
        assert(std::abs(actual[index] - expected[index]) < 1.0E-6F);
    }
}

} // namespace

int main() {
    {
        const echo::audio::PreparedAdjustment prepared(master_only(), 1000, kSampleRate);
        echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
        assert(chain.latency_frames() == 0);
        const auto input = fixture(23);
        const auto output = process_in_chunks(chain, input, 7);
        assert_near(output, input);
    }

    {
        constexpr std::array<std::size_t, 5> frame_counts = {31, 96, 97, 98, 151};
        constexpr std::array<std::size_t, 4> chunk_sizes = {1, 13, 64, 101};
        for (const std::size_t frame_count : frame_counts) {
            for (const std::size_t chunk_size : chunk_sizes) {
                const echo::audio::PreparedAdjustment prepared(
                    bypassed_de_click(),
                    1000,
                    kSampleRate
                );
                echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
                assert(chain.latency_frames() == 97);
                const auto input = fixture(frame_count);
                const auto output = process_in_chunks(chain, input, chunk_size);
                assert_near(output, input);
                assert(chain.pending_output_frames() == 0);

                chain.reset();
                const auto reset_output = process_in_chunks(chain, input, chunk_size);
                assert_near(reset_output, input);
            }
        }
    }

    {
        const auto input = fixture(151);
        const echo::audio::PreparedAdjustment first(bypassed_de_click(), 1000, kSampleRate);
        const echo::audio::PreparedAdjustment second(
            reordered_bypassed_de_click(),
            1000,
            kSampleRate
        );
        echo::audio::EffectProcessingChain first_chain(first, kSampleRate, kChannels);
        echo::audio::EffectProcessingChain second_chain(second, kSampleRate, kChannels);
        assert(first_chain.latency_frames() == second_chain.latency_frames());
        assert_near(process_in_chunks(first_chain, input, 23), input);
        assert_near(process_in_chunks(second_chain, input, 23), input);
    }

    {
        auto adjustment = master_only();
        adjustment.effect_chain = {
            echo::audio::EffectNodeKind::Equalizer,
            echo::audio::EffectNodeKind::Master,
            echo::audio::EffectNodeKind::Restoration,
            echo::audio::EffectNodeKind::Dynamics,
            echo::audio::EffectNodeKind::Space,
            echo::audio::EffectNodeKind::DeHum,
            echo::audio::EffectNodeKind::DeClick,
        };
        adjustment.effect_chain_count = 2;
        adjustment.equalizer.bands[0].gain_centibels = 1200;
        adjustment.effect_masks = {{
            .start_millis = 0,
            .end_millis = 1,
            .nodes = {echo::audio::EffectNodeKind::Equalizer},
        }};
        const echo::audio::PreparedAdjustment prepared(adjustment, 1000, kSampleRate);
        const echo::audio::EffectMaskPlan mask_plan(adjustment, prepared, kSampleRate);
        echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels, &mask_plan);
        const auto input = fixture(96);
        auto output = input;
        std::array<std::uint64_t, 96> anchors{};
        for (std::size_t frame = 0; frame < anchors.size(); ++frame) {
            anchors[frame] = frame;
        }
        const std::size_t produced =
            chain.process_interleaved(output.data(), anchors.data(), anchors.size(), kChannels);
        assert(produced == anchors.size());
        bool wet_changed = false;
        for (std::size_t frame = 0; frame < 48; ++frame) {
            wet_changed =
                wet_changed
                || std::abs(output[frame * kChannels] - input[frame * kChannels]) > 1.0E-6F;
        }
        assert(wet_changed);
        for (std::size_t frame = 48; frame < anchors.size(); ++frame) {
            assert(std::abs(output[frame * kChannels] - input[frame * kChannels]) < 1.0E-6F);
        }
    }

    return 0;
}
