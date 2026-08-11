#include "echo/audio/scene_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <new>
#include <stdexcept>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;
constexpr std::size_t kChannels = 2;
std::atomic<bool> allocation_tracking = false;
std::atomic<std::size_t> allocation_count = 0;

echo::audio::SceneVfxAdjustment enabled(echo::audio::SceneVfxCharacter character) {
    return {
        .character = character,
        .enabled = true,
        .mix_percent = 100,
        .intensity_percent = 70,
    };
}

std::vector<float> fixture(std::size_t frame_count, std::size_t channel_count = kChannels) {
    std::vector<float> samples(frame_count * channel_count);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float phase = static_cast<float>(frame) * 0.037F;
        samples[frame * channel_count] = 0.55F * std::sin(phase) + 0.17F * std::sin(phase * 7.3F);
        if (channel_count == 2) {
            samples[frame * channel_count + 1] =
                -0.31F * std::cos(phase * 1.7F) + 0.11F * std::sin(phase * 11.0F);
        }
    }
    return samples;
}

std::vector<float> process(
    echo::audio::SceneVfxAdjustment adjustment,
    const std::vector<float>& input,
    std::size_t channel_count,
    std::size_t block_frames
) {
    echo::audio::SceneVfxProcessor processor(adjustment, kSampleRate, channel_count);
    std::vector<float> output = input;
    const std::size_t frame_count = output.size() / channel_count;
    std::size_t offset = 0;
    while (offset < frame_count) {
        const std::size_t frames = std::min(block_frames, frame_count - offset);
        processor
            .process_interleaved(output.data() + offset * channel_count, frames, channel_count);
        offset += frames;
    }
    return output;
}

void assert_near(const std::vector<float>& actual, const std::vector<float>& expected) {
    assert(actual.size() == expected.size());
    for (std::size_t index = 0; index < actual.size(); ++index) {
        assert(std::abs(actual[index] - expected[index]) < 1.0E-6F);
    }
}

void assert_finite(const std::vector<float>& samples) {
    assert(std::all_of(samples.begin(), samples.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) < 8.0F;
    }));
}

} // namespace

void* operator new(std::size_t size) {
    if (allocation_tracking.load(std::memory_order_relaxed)) {
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
    const auto input = fixture(4096);

    {
        echo::audio::SceneVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(output == input);
        assert(processor.is_bypassed());
        assert(processor.latency_frames() == 0);
    }

    {
        auto adjustment = enabled(echo::audio::SceneVfxCharacter::Radio);
        adjustment.mix_percent = 0;
        assert(process(adjustment, input, kChannels, 257) == input);
        adjustment.mix_percent = 100;
        adjustment.intensity_percent = 0;
        assert(process(adjustment, input, kChannels, 257) == input);
    }

    constexpr std::array characters{
        echo::audio::SceneVfxCharacter::Telephone,
        echo::audio::SceneVfxCharacter::Radio,
        echo::audio::SceneVfxCharacter::Intercom,
        echo::audio::SceneVfxCharacter::BehindWall,
        echo::audio::SceneVfxCharacter::Underwater,
    };
    std::array<std::vector<float>, characters.size()> character_outputs;
    for (std::size_t index = 0; index < characters.size(); ++index) {
        const auto adjustment = enabled(characters[index]);
        character_outputs[index] = process(adjustment, input, kChannels, 257);
        assert_finite(character_outputs[index]);
        const auto sampled = process(adjustment, input, kChannels, 1);
        const auto blocked = process(adjustment, input, kChannels, 13);
        assert_near(sampled, blocked);
        assert_near(sampled, character_outputs[index]);

        const auto mono_input = fixture(2048, 1);
        const auto mono_output = process(adjustment, mono_input, 1, 37);
        assert_finite(mono_output);
        assert(mono_output != mono_input);

        std::vector<float> silence(2048 * kChannels, 0.0F);
        const auto silent_output = process(adjustment, silence, kChannels, 31);
        assert(std::all_of(silent_output.begin(), silent_output.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }
    for (std::size_t left = 0; left < character_outputs.size(); ++left) {
        for (std::size_t right = left + 1; right < character_outputs.size(); ++right) {
            bool differs = false;
            for (std::size_t index = 0; index < input.size(); ++index) {
                differs =
                    differs
                    || std::abs(character_outputs[left][index] - character_outputs[right][index])
                           > 1.0E-5F;
            }
            assert(differs);
        }
    }

    {
        const auto telephone = enabled(echo::audio::SceneVfxCharacter::Telephone);
        const auto underwater = enabled(echo::audio::SceneVfxCharacter::Underwater);
        echo::audio::SceneVfxProcessor transitioning(telephone, kSampleRate, kChannels);
        echo::audio::SceneVfxProcessor reference(telephone, kSampleRate, kChannels);
        auto warmup_a = fixture(512);
        auto warmup_b = warmup_a;
        transitioning.process_interleaved(warmup_a.data(), 512, kChannels);
        reference.process_interleaved(warmup_b.data(), 512, kChannels);
        transitioning.update(underwater);
        std::array<float, 2> next_a{{0.35F, -0.2F}};
        auto next_b = next_a;
        transitioning.process_interleaved(next_a.data(), 1, kChannels);
        reference.process_interleaved(next_b.data(), 1, kChannels);
        assert(std::abs(next_a[0] - next_b[0]) < 0.01F);
        assert(std::abs(next_a[1] - next_b[1]) < 0.01F);

        auto tail = fixture(2048);
        transitioning.process_interleaved(tail.data(), tail.size() / kChannels, kChannels);
        assert_finite(tail);
    }

    {
        const auto adjustment = enabled(echo::audio::SceneVfxCharacter::Underwater);
        echo::audio::SceneVfxProcessor used(adjustment, kSampleRate, kChannels);
        echo::audio::SceneVfxProcessor fresh(adjustment, kSampleRate, kChannels);
        auto discarded = fixture(3000);
        used.process_interleaved(discarded.data(), discarded.size() / kChannels, kChannels);
        used.reset();
        auto reset_output = input;
        auto fresh_output = input;
        used.process_interleaved(reset_output.data(), reset_output.size() / kChannels, kChannels);
        fresh.process_interleaved(fresh_output.data(), fresh_output.size() / kChannels, kChannels);
        assert_near(reset_output, fresh_output);
    }

    {
        bool rejected = false;
        try {
            auto invalid = enabled(echo::audio::SceneVfxCharacter::Telephone);
            invalid.character = static_cast<echo::audio::SceneVfxCharacter>(255);
            echo::audio::SceneVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            auto invalid = enabled(echo::audio::SceneVfxCharacter::Telephone);
            invalid.mix_percent = 101;
            echo::audio::SceneVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            auto invalid = enabled(echo::audio::SceneVfxCharacter::Telephone);
            invalid.intensity_percent = 101;
            echo::audio::SceneVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            echo::audio::SceneVfxProcessor processor({}, kSampleRate, 3);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        echo::audio::SceneVfxProcessor processor({}, kSampleRate, kChannels);
        rejected = false;
        try {
            processor.process_interleaved(nullptr, 1, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    {
        auto adjustment = enabled(echo::audio::SceneVfxCharacter::Telephone);
        echo::audio::SceneVfxProcessor processor(adjustment, kSampleRate, kChannels);
        std::array<float, 1024> samples{};
        for (std::size_t index = 0; index < samples.size(); ++index) {
            samples[index] = 0.25F * std::sin(static_cast<float>(index) * 0.02F);
        }
        allocation_count.store(0, std::memory_order_relaxed);
        allocation_tracking.store(true, std::memory_order_relaxed);
        adjustment.character = echo::audio::SceneVfxCharacter::Underwater;
        processor.update(adjustment);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        processor.reset();
        allocation_tracking.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    return 0;
}
