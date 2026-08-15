#include "echo/audio/effect_processing_chain.hpp"

#include <algorithm>
#include <array>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <new>
#include <stdexcept>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;
constexpr std::size_t kChannels = 2;
std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocation_count = 0;

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
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
        echo::audio::EffectNodeKind::TapeVfx,
        echo::audio::EffectNodeKind::PitchVfx,
        echo::audio::EffectNodeKind::AutoWahVfx,
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
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
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
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    return adjustment;
}

echo::audio::PlaybackAdjustment channel_repair_only() {
    auto adjustment = master_only();
    adjustment.channel_repair = {
        .enabled = true,
        .invert_left = true,
        .swap_channels = true,
        .balance_percent = 50,
    };
    adjustment.effect_chain = {
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    adjustment.effect_chain_count = 2;
    return adjustment;
}

echo::audio::PlaybackAdjustment space_character(echo::audio::ReverbCharacter character) {
    auto adjustment = master_only();
    adjustment.reverb = {
        .character = character,
        .enabled = true,
        .mix_percent = 65,
        .pre_delay_millis = 8,
        .decay_millis = 2'600,
        .size_percent = 72,
        .damping_percent = 38,
        .low_cut_hertz = 120,
        .high_cut_hertz = 10'000,
    };
    adjustment.effect_chain = {
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    adjustment.effect_chain_count = 2;
    return adjustment;
}

echo::audio::PlaybackAdjustment creative_only(echo::audio::EffectNodeKind node) {
    auto adjustment = master_only();
    auto node_iterator =
        std::find(adjustment.effect_chain.begin(), adjustment.effect_chain.end(), node);
    assert(node_iterator != adjustment.effect_chain.end());
    std::iter_swap(adjustment.effect_chain.begin(), node_iterator);
    auto master_iterator = std::find(
        adjustment.effect_chain.begin() + 1,
        adjustment.effect_chain.end(),
        echo::audio::EffectNodeKind::Master
    );
    assert(master_iterator != adjustment.effect_chain.end());
    std::iter_swap(adjustment.effect_chain.begin() + 1, master_iterator);
    adjustment.effect_chain_count = 2;
    switch (node) {
    case echo::audio::EffectNodeKind::SceneVfx:
        adjustment.creative_vfx.scene.enabled = true;
        adjustment.creative_vfx.scene.character = echo::audio::SceneVfxCharacter::Radio;
        break;
    case echo::audio::EffectNodeKind::DelayVfx:
        adjustment.creative_vfx.delay.enabled = true;
        adjustment.creative_vfx.delay.character = echo::audio::DelayVfxCharacter::Echo;
        break;
    case echo::audio::EffectNodeKind::ModulationVfx:
        adjustment.creative_vfx.modulation.enabled = true;
        adjustment.creative_vfx.modulation.character = echo::audio::ModulationVfxCharacter::Phaser;
        break;
    case echo::audio::EffectNodeKind::TransformVfx:
        adjustment.creative_vfx.transform.enabled = true;
        adjustment.creative_vfx.transform.character = echo::audio::TransformVfxCharacter::Robot;
        break;
    case echo::audio::EffectNodeKind::DigitalDegradeVfx:
        adjustment.creative_vfx.digital_degrade.enabled = true;
        adjustment.creative_vfx.digital_degrade.character =
            echo::audio::DigitalDegradeVfxCharacter::LoFi;
        break;
    case echo::audio::EffectNodeKind::DriveVfx:
        adjustment.creative_vfx.drive.enabled = true;
        adjustment.creative_vfx.drive.character = echo::audio::DriveVfxCharacter::Overdrive;
        break;
    case echo::audio::EffectNodeKind::RotaryVfx:
        adjustment.creative_vfx.rotary.enabled = true;
        adjustment.creative_vfx.rotary.speed = echo::audio::RotaryVfxSpeed::Fast;
        break;
    case echo::audio::EffectNodeKind::FreezeVfx:
        adjustment.creative_vfx.freeze.enabled = true;
        adjustment.creative_vfx.freeze.capture_source_millis = 100;
        break;
    case echo::audio::EffectNodeKind::GranularVfx:
        adjustment.creative_vfx.granular.enabled = true;
        break;
    case echo::audio::EffectNodeKind::TapeVfx:
        adjustment.creative_vfx.tape.enabled = true;
        break;
    case echo::audio::EffectNodeKind::PitchVfx:
        adjustment.creative_vfx.pitch.enabled = true;
        break;
    case echo::audio::EffectNodeKind::AutoWahVfx:
        adjustment.creative_vfx.auto_wah.enabled = true;
        break;
    case echo::audio::EffectNodeKind::Restoration:
    case echo::audio::EffectNodeKind::Equalizer:
    case echo::audio::EffectNodeKind::Dynamics:
    case echo::audio::EffectNodeKind::Space:
    case echo::audio::EffectNodeKind::Master:
    case echo::audio::EffectNodeKind::DeHum:
    case echo::audio::EffectNodeKind::DeClick:
    case echo::audio::EffectNodeKind::ChannelRepair:
        assert(false);
        break;
    }
    return adjustment;
}

echo::audio::PlaybackAdjustment two_latency_nodes(bool transform_first) {
    auto adjustment = master_only();
    adjustment.effect_chain = {
        transform_first ? echo::audio::EffectNodeKind::TransformVfx
                        : echo::audio::EffectNodeKind::DeClick,
        transform_first ? echo::audio::EffectNodeKind::DeClick
                        : echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    adjustment.effect_chain_count = 3;
    return adjustment;
}

echo::audio::PlaybackAdjustment freeze_and_granular(bool freeze, bool granular) {
    auto adjustment = master_only();
    adjustment.trim_end_millis = 3000;
    adjustment.creative_vfx.freeze.enabled = freeze;
    adjustment.creative_vfx.freeze.mix_percent = 70;
    adjustment.creative_vfx.freeze.capture_source_millis = 100;
    adjustment.creative_vfx.granular.enabled = granular;
    adjustment.creative_vfx.granular.mix_percent = 65;
    adjustment.effect_chain = {
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
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
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
    };
    if (freeze && granular) {
        adjustment.effect_chain_count = 3;
    } else if (freeze) {
        adjustment.effect_chain[1] = echo::audio::EffectNodeKind::Master;
        adjustment.effect_chain[2] = echo::audio::EffectNodeKind::GranularVfx;
        adjustment.effect_chain_count = 2;
    } else {
        adjustment.effect_chain[0] = echo::audio::EffectNodeKind::GranularVfx;
        adjustment.effect_chain[1] = echo::audio::EffectNodeKind::Master;
        adjustment.effect_chain[2] = echo::audio::EffectNodeKind::FreezeVfx;
        adjustment.effect_chain_count = 2;
    }
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

std::vector<float> process_source_aware_in_chunks(
    echo::audio::EffectProcessingChain& chain,
    const std::vector<float>& input,
    std::size_t chunk_frames,
    std::uint64_t first_source_frame = 0
) {
    const std::size_t input_frames = input.size() / kChannels;
    std::vector<float> output;
    std::vector<float> scratch(chunk_frames * kChannels);
    std::vector<std::uint64_t> anchors(chunk_frames, echo::audio::kNoSourceFrame);
    std::size_t consumed = 0;
    while (consumed < input_frames) {
        const std::size_t frames = std::min(chunk_frames, input_frames - consumed);
        std::copy_n(input.data() + consumed * kChannels, frames * kChannels, scratch.data());
        for (std::size_t frame = 0; frame < frames; ++frame) {
            anchors[frame] = first_source_frame + consumed + frame;
        }
        const std::size_t produced =
            chain.process_interleaved(scratch.data(), anchors.data(), frames, kChannels);
        output.insert(output.end(), scratch.data(), scratch.data() + produced * kChannels);
        consumed += frames;
    }
    while (chain.pending_output_frames() > 0) {
        const std::size_t produced =
            chain.finish_interleaved(scratch.data(), anchors.data(), chunk_frames, kChannels);
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

void* operator new(std::size_t size) {
    if (track_allocations.load(std::memory_order_relaxed)) {
        allocation_count.fetch_add(1, std::memory_order_relaxed);
    }
    if (void* memory = std::malloc(size)) {
        return memory;
    }
    throw std::bad_alloc();
}

void* operator new[](std::size_t size) {
    return ::operator new(size);
}

void operator delete(void* memory) noexcept {
    std::free(memory);
}

void operator delete[](void* memory) noexcept {
    std::free(memory);
}

void operator delete(void* memory, std::size_t) noexcept {
    std::free(memory);
}

void operator delete[](void* memory, std::size_t) noexcept {
    std::free(memory);
}

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
        const echo::audio::PreparedAdjustment prepared(channel_repair_only(), 1000, kSampleRate);
        echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
        const std::vector<float> input = {0.8F, -0.2F};
        const auto output = process_in_chunks(chain, input, 1);
        assert(output.size() == 2);
        assert(std::abs(output[0] - -0.1F) < 1.0E-6F);
        assert(std::abs(output[1] - -0.8F) < 1.0E-6F);
    }

    {
        const auto input = fixture(8'192);
        const echo::audio::PreparedAdjustment hall_prepared(
            space_character(echo::audio::ReverbCharacter::Hall),
            1000,
            kSampleRate
        );
        const echo::audio::PreparedAdjustment plate_prepared(
            space_character(echo::audio::ReverbCharacter::Plate),
            1000,
            kSampleRate
        );
        const echo::audio::PreparedAdjustment spring_prepared(
            space_character(echo::audio::ReverbCharacter::Spring),
            1000,
            kSampleRate
        );
        echo::audio::EffectProcessingChain hall_sampled(hall_prepared, kSampleRate, kChannels);
        echo::audio::EffectProcessingChain hall_blocked(hall_prepared, kSampleRate, kChannels);
        echo::audio::EffectProcessingChain plate(plate_prepared, kSampleRate, kChannels);
        echo::audio::EffectProcessingChain spring(spring_prepared, kSampleRate, kChannels);
        assert(hall_sampled.latency_frames() == 0);
        assert(plate.latency_frames() == 0);
        assert(spring.latency_frames() == 0);
        const auto hall_one = process_in_chunks(hall_sampled, input, 1);
        const auto hall_257 = process_in_chunks(hall_blocked, input, 257);
        const auto plate_64 = process_in_chunks(plate, input, 64);
        const auto spring_64 = process_in_chunks(spring, input, 64);
        assert_near(hall_one, hall_257);
        bool hall_changed = false;
        bool characters_differ = false;
        for (std::size_t index = 0; index < input.size(); ++index) {
            hall_changed = hall_changed || std::abs(hall_one[index] - input[index]) > 1.0E-6F;
            characters_differ = characters_differ
                                || (std::abs(hall_one[index] - plate_64[index]) > 1.0E-6F
                                    && std::abs(plate_64[index] - spring_64[index]) > 1.0E-6F);
        }
        assert(hall_changed);
        assert(characters_differ);
    }

    {
        const auto input = fixture(24'000);
        const echo::audio::PreparedAdjustment prepared(
            freeze_and_granular(true, false),
            3000,
            kSampleRate
        );
        echo::audio::EffectProcessingChain sampled(prepared, kSampleRate, kChannels);
        echo::audio::EffectProcessingChain blocked(prepared, kSampleRate, kChannels);
        assert(sampled.latency_frames() == 4096);
        const auto one = process_source_aware_in_chunks(sampled, input, 1);
        const auto two_fifty_seven = process_source_aware_in_chunks(blocked, input, 257);
        assert_near(one, two_fifty_seven);
        assert(one.size() == input.size());
        bool changed = false;
        for (std::size_t index = 0; index < one.size(); ++index) {
            changed = changed || std::abs(one[index] - input[index]) > 1.0E-6F;
        }
        assert(changed);

        echo::audio::EffectProcessingChain missing_anchors(prepared, kSampleRate, kChannels);
        auto rejected_input = fixture(8'192);
        bool rejected = false;
        try {
            missing_anchors.process_interleaved(
                rejected_input.data(),
                rejected_input.size() / kChannels,
                kChannels
            );
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        auto changed_anchor = prepared.creative_vfx().freeze;
        ++changed_anchor.capture_source_millis;
        rejected = false;
        try {
            blocked.update_freeze_vfx(changed_anchor);
        } catch (const std::logic_error&) {
            rejected = true;
        }
        assert(rejected);
    }

    {
        const auto input = fixture(96'000);
        const echo::audio::PreparedAdjustment prepared(
            freeze_and_granular(false, true),
            3000,
            kSampleRate
        );
        echo::audio::EffectProcessingChain sampled(prepared, kSampleRate, kChannels);
        echo::audio::EffectProcessingChain blocked(prepared, kSampleRate, kChannels);
        assert(sampled.latency_frames() == 0);
        const auto one = process_in_chunks(sampled, input, 1);
        const auto two_fifty_seven = process_in_chunks(blocked, input, 257);
        assert_near(one, two_fifty_seven);
        bool changed = false;
        for (std::size_t index = 0; index < one.size(); ++index) {
            changed = changed || std::abs(one[index] - input[index]) > 1.0E-6F;
        }
        assert(changed);
    }

    {
        auto authored_but_inactive = master_only();
        authored_but_inactive.trim_end_millis = 3000;
        authored_but_inactive.creative_vfx.freeze.enabled = true;
        authored_but_inactive.creative_vfx.freeze.capture_source_millis = 100;
        const echo::audio::PreparedAdjustment prepared(authored_but_inactive, 3000, kSampleRate);
        echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
        const auto input = fixture(8'192);
        assert(chain.latency_frames() == 0);
        assert_near(process_in_chunks(chain, input, 257), input);
    }

    {
        auto delayed_anchor = master_only();
        delayed_anchor.trim_end_millis = 1000;
        delayed_anchor.creative_vfx.freeze.enabled = true;
        delayed_anchor.creative_vfx.freeze.capture_source_millis = 990;
        delayed_anchor.effect_chain = {
            echo::audio::EffectNodeKind::TransformVfx,
            echo::audio::EffectNodeKind::FreezeVfx,
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
            echo::audio::EffectNodeKind::DriveVfx,
            echo::audio::EffectNodeKind::RotaryVfx,
            echo::audio::EffectNodeKind::TapeVfx,
            echo::audio::EffectNodeKind::PitchVfx,
            echo::audio::EffectNodeKind::AutoWahVfx,
            echo::audio::EffectNodeKind::GranularVfx,
        };
        delayed_anchor.effect_chain_count = 3;
        const echo::audio::PreparedAdjustment prepared(delayed_anchor, 1000, kSampleRate);
        echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
        const auto input = fixture(48'000);
        const auto output = process_source_aware_in_chunks(chain, input, 257);
        assert(output.size() == input.size());
    }

    {
        const echo::audio::PreparedAdjustment prepared(
            freeze_and_granular(true, true),
            3000,
            kSampleRate
        );
        echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
        auto input = fixture(8'192);
        std::vector<std::uint64_t> anchors(8'192);
        for (std::size_t frame = 0; frame < anchors.size(); ++frame) {
            anchors[frame] = frame;
        }
        auto freeze_update = prepared.creative_vfx().freeze;
        freeze_update.mix_percent = 40;
        auto granular_update = prepared.creative_vfx().granular;
        granular_update.pitch_cents = -300;
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        chain.update_freeze_vfx(freeze_update);
        chain.update_granular_vfx(granular_update);
        chain.process_interleaved(input.data(), anchors.data(), anchors.size(), kChannels);
        chain.reset();
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    {
        const auto input = fixture(8'192);
        constexpr std::array creative_nodes{
            echo::audio::EffectNodeKind::SceneVfx,
            echo::audio::EffectNodeKind::DelayVfx,
            echo::audio::EffectNodeKind::ModulationVfx,
            echo::audio::EffectNodeKind::TransformVfx,
            echo::audio::EffectNodeKind::DigitalDegradeVfx,
            echo::audio::EffectNodeKind::DriveVfx,
            echo::audio::EffectNodeKind::RotaryVfx,
        };
        for (const auto node : creative_nodes) {
            const echo::audio::PreparedAdjustment prepared(creative_only(node), 1000, kSampleRate);
            echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
            assert(
                chain.latency_frames()
                == (node == echo::audio::EffectNodeKind::TransformVfx
                        ? 2'400
                        : (node == echo::audio::EffectNodeKind::DriveVfx
                               ? 32
                               : (node == echo::audio::EffectNodeKind::PitchVfx ? 576 : 0)))
            );
            const auto output = process_in_chunks(chain, input, 137);
            assert(output.size() == input.size());
            bool changed = false;
            for (std::size_t index = 0; index < input.size(); ++index) {
                assert(std::isfinite(output[index]));
                changed = changed || std::abs(output[index] - input[index]) > 1.0E-6F;
            }
            assert(changed);
        }
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
        const auto input = fixture(5'123);
        for (const bool transform_first : {false, true}) {
            const echo::audio::PreparedAdjustment prepared(
                two_latency_nodes(transform_first),
                1000,
                kSampleRate
            );
            echo::audio::EffectProcessingChain chain(prepared, kSampleRate, kChannels);
            assert(chain.latency_frames() == 2'497);
            assert_near(process_in_chunks(chain, input, 113), input);
        }
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
            echo::audio::EffectNodeKind::ChannelRepair,
            echo::audio::EffectNodeKind::SceneVfx,
            echo::audio::EffectNodeKind::DelayVfx,
            echo::audio::EffectNodeKind::ModulationVfx,
            echo::audio::EffectNodeKind::TransformVfx,
            echo::audio::EffectNodeKind::DigitalDegradeVfx,
            echo::audio::EffectNodeKind::DriveVfx,
            echo::audio::EffectNodeKind::RotaryVfx,
            echo::audio::EffectNodeKind::FreezeVfx,
            echo::audio::EffectNodeKind::GranularVfx,
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
