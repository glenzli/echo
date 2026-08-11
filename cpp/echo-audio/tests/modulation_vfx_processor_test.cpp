#include "echo/audio/modulation_vfx_processor.hpp"

#include <algorithm>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <new>
#include <stdexcept>
#include <vector>

namespace {

std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocation_count = 0;

constexpr std::uint32_t kSampleRate = 48000;
constexpr float kPi = 3.14159265358979323846F;

echo::audio::ModulationVfxAdjustment enabled(echo::audio::ModulationVfxCharacter character) {
    echo::audio::ModulationVfxAdjustment value;
    value.enabled = true;
    value.character = character;
    return value;
}

std::vector<float> stereo_program(std::size_t frame_count) {
    std::vector<float> samples(frame_count * 2, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * 2] = 0.27F * std::sin(2.0F * kPi * 523.0F * time);
        samples[frame * 2 + 1] = 0.21F * std::sin(2.0F * kPi * 811.0F * time + 0.31F);
    }
    return samples;
}

float absolute_difference(const std::vector<float>& left, const std::vector<float>& right) {
    assert(left.size() == right.size());
    float difference = 0.0F;
    for (std::size_t index = 0; index < left.size(); ++index) {
        difference += std::abs(left[index] - right[index]);
    }
    return difference;
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
    static_assert(echo::audio::ModulationVfxProcessor::latency_frames() == 0);
    {
        auto input = stereo_program(12000);
        auto bypassed = input;
        echo::audio::ModulationVfxProcessor processor({}, kSampleRate, 2);
        assert(processor.is_bypassed());
        processor.process_interleaved(bypassed.data(), bypassed.size() / 2, 2);
        assert(bypassed == input);
    }

    {
        auto chorus = enabled(echo::audio::ModulationVfxCharacter::Chorus);
        chorus.chorus.mix_percent = 100;
        chorus.chorus.rate_millihertz = 50;
        chorus.chorus.minimum_delay_microseconds = 8000;
        chorus.chorus.sweep_microseconds = 500;
        std::vector<float> impulse(2000, 0.0F);
        impulse[0] = 1.0F;
        echo::audio::ModulationVfxProcessor processor(chorus, kSampleRate, 1);
        processor.process_interleaved(impulse.data(), impulse.size(), 1);
        for (std::size_t frame = 0; frame < 380; ++frame) {
            assert(impulse[frame] == 0.0F);
        }
        float delayed_energy = 0.0F;
        for (std::size_t frame = 380; frame < 430; ++frame) {
            delayed_energy += std::abs(impulse[frame]);
        }
        assert(delayed_energy > 0.8F);
    }

    {
        auto tremolo = enabled(echo::audio::ModulationVfxCharacter::Tremolo);
        tremolo.tremolo.rate_millihertz = 4000;
        tremolo.tremolo.depth_percent = 100;
        tremolo.tremolo.stereo_phase_degrees = 180;
        std::vector<float> constant(10000 * 2, 1.0F);
        echo::audio::ModulationVfxProcessor processor(tremolo, kSampleRate, 2);
        processor.process_interleaved(constant.data(), constant.size() / 2, 2);
        constexpr std::size_t quarter_period = kSampleRate / 16;
        assert(constant[quarter_period * 2] < 0.01F);
        assert(constant[quarter_period * 2 + 1] > 0.99F);
        assert(constant[quarter_period * 6] > 0.99F);
        assert(constant[quarter_period * 6 + 1] < 0.01F);
    }

    const auto input = stereo_program(24000);
    {
        for (const auto character : {
                 echo::audio::ModulationVfxCharacter::Chorus,
                 echo::audio::ModulationVfxCharacter::Flanger,
                 echo::audio::ModulationVfxCharacter::Phaser,
                 echo::audio::ModulationVfxCharacter::Tremolo,
             }) {
            auto processed = input;
            echo::audio::ModulationVfxProcessor processor(enabled(character), kSampleRate, 2);
            processor.process_interleaved(processed.data(), processed.size() / 2, 2);
            assert(absolute_difference(processed, input) > 1.0F);
            for (const float sample : processed) {
                assert(std::isfinite(sample));
                assert(std::abs(sample) < 4.0F);
            }
        }
    }

    {
        auto contiguous = input;
        auto blocked = input;
        const auto adjustment = enabled(echo::audio::ModulationVfxCharacter::Flanger);
        echo::audio::ModulationVfxProcessor first(adjustment, kSampleRate, 2);
        echo::audio::ModulationVfxProcessor second(adjustment, kSampleRate, 2);
        first.process_interleaved(contiguous.data(), contiguous.size() / 2, 2);
        std::size_t cursor = 0;
        while (cursor < blocked.size() / 2) {
            const std::size_t frames =
                std::min<std::size_t>(71 + cursor % 311, blocked.size() / 2 - cursor);
            second.process_interleaved(blocked.data() + cursor * 2, frames, 2);
            cursor += frames;
        }
        assert(contiguous == blocked);
    }

    {
        auto first = input;
        auto replay = input;
        const auto adjustment = enabled(echo::audio::ModulationVfxCharacter::Phaser);
        echo::audio::ModulationVfxProcessor processor(adjustment, kSampleRate, 2);
        processor.process_interleaved(first.data(), first.size() / 2, 2);
        processor.reset();
        processor.process_interleaved(replay.data(), replay.size() / 2, 2);
        assert(first == replay);
    }

    {
        auto probe = input;
        echo::audio::ModulationVfxProcessor processor(
            enabled(echo::audio::ModulationVfxCharacter::Chorus),
            kSampleRate,
            2
        );
        auto target = enabled(echo::audio::ModulationVfxCharacter::Phaser);
        target.phaser.feedback_percent = -60;
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        processor.update(target);
        processor.process_interleaved(probe.data(), probe.size() / 2, 2);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
        float maximum_step = 0.0F;
        for (std::size_t frame = 1; frame < probe.size() / 2; ++frame) {
            maximum_step =
                std::max(maximum_step, std::abs(probe[frame * 2] - probe[(frame - 1) * 2]));
        }
        assert(maximum_step < 0.25F);
    }

    {
        bool rejected = false;
        auto invalid = enabled(echo::audio::ModulationVfxCharacter::Phaser);
        invalid.phaser.sweep_low_hertz = invalid.phaser.sweep_high_hertz;
        try {
            echo::audio::ModulationVfxProcessor processor(invalid, kSampleRate, 2);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    return 0;
}
