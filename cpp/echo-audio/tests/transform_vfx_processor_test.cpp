#include "echo/audio/transform_vfx_processor.hpp"

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

std::atomic<bool> track_allocations{false};
std::atomic<std::size_t> allocation_count{0};

constexpr std::uint32_t kSampleRate = 48000;
constexpr float kPi = 3.14159265358979323846F;

echo::audio::TransformVfxAdjustment enabled(echo::audio::TransformVfxCharacter character) {
    return {
        .character = character,
        .enabled = true,
        .mix_percent = 100,
        .amount_percent = 65,
    };
}

std::vector<float> test_signal(std::size_t frame_count, std::size_t channels) {
    std::vector<float> samples(frame_count * channels, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        const float source = 0.42F * std::sin(2.0F * kPi * 220.0F * time)
                             + 0.17F * std::sin(2.0F * kPi * 670.0F * time);
        for (std::size_t channel = 0; channel < channels; ++channel) {
            samples[frame * channels + channel] = channel == 0 ? source : source * 0.73F;
        }
    }
    return samples;
}

double mean_difference(
    const std::vector<float>& first,
    const std::vector<float>& second,
    std::size_t begin
) {
    double sum = 0.0;
    for (std::size_t index = begin; index < first.size(); ++index) {
        sum += std::abs(static_cast<double>(first[index] - second[index]));
    }
    return sum / static_cast<double>(first.size() - begin);
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
        bool rejected = false;
        try {
            echo::audio::TransformVfxProcessor invalid({.mix_percent = 101}, kSampleRate, 2);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    constexpr std::size_t kFrames = 18000;
    const auto mono_input = test_signal(kFrames, 1);
    {
        auto bypassed = mono_input;
        echo::audio::TransformVfxProcessor processor({}, kSampleRate, 1);
        assert(processor.latency_frames() == 2400);
        processor.process_interleaved(bypassed.data(), bypassed.size(), 1);
        for (std::size_t frame = 0; frame < processor.latency_frames(); ++frame) {
            assert(bypassed[frame] == 0.0F);
        }
        for (std::size_t frame = processor.latency_frames(); frame < bypassed.size(); ++frame) {
            assert(bypassed[frame] == mono_input[frame - processor.latency_frames()]);
        }
    }
    {
        echo::audio::TransformVfxProcessor scaled({}, 96000, 2);
        assert(scaled.latency_frames() == 4800);

        auto zero_mix = mono_input;
        echo::audio::TransformVfxProcessor processor(
            {
                .character = echo::audio::TransformVfxCharacter::Monster,
                .enabled = true,
                .mix_percent = 0,
                .amount_percent = 100,
            },
            kSampleRate,
            1
        );
        processor.process_interleaved(zero_mix.data(), zero_mix.size(), 1);
        for (std::size_t frame = processor.latency_frames(); frame < zero_mix.size(); ++frame) {
            assert(zero_mix[frame] == mono_input[frame - processor.latency_frames()]);
        }
    }

    const std::array characters{
        echo::audio::TransformVfxCharacter::Robot,
        echo::audio::TransformVfxCharacter::Monster,
        echo::audio::TransformVfxCharacter::Tiny,
        echo::audio::TransformVfxCharacter::Giant,
        echo::audio::TransformVfxCharacter::Ghost,
    };
    std::array<std::vector<float>, 5> rendered;
    for (std::size_t index = 0; index < characters.size(); ++index) {
        rendered[index] = mono_input;
        echo::audio::TransformVfxProcessor processor(enabled(characters[index]), kSampleRate, 1);
        assert(processor.latency_frames() == 2400);
        processor.process_interleaved(rendered[index].data(), rendered[index].size(), 1);
        for (std::size_t frame = 0; frame < rendered[index].size(); ++frame) {
            const float sample = rendered[index][frame];
            assert(std::isfinite(sample));
            assert(std::abs(sample) < 2.0F);
            if (frame < processor.latency_frames()) {
                assert(sample == 0.0F);
            }
        }
    }
    for (std::size_t first = 0; first < rendered.size(); ++first) {
        for (std::size_t second = first + 1; second < rendered.size(); ++second) {
            assert(mean_difference(rendered[first], rendered[second], 6000) > 0.025);
        }
    }

    {
        auto contiguous = test_signal(kFrames, 2);
        auto chunked = contiguous;
        echo::audio::TransformVfxProcessor first(
            enabled(echo::audio::TransformVfxCharacter::Tiny),
            kSampleRate,
            2
        );
        echo::audio::TransformVfxProcessor second(
            enabled(echo::audio::TransformVfxCharacter::Tiny),
            kSampleRate,
            2
        );
        first.process_interleaved(contiguous.data(), kFrames, 2);
        std::size_t cursor = 0;
        while (cursor < kFrames) {
            const std::size_t count = std::min<std::size_t>(
                cursor % 3 == 0 ? 1 : (cursor % 3 == 1 ? 137 : 4096),
                kFrames - cursor
            );
            second.process_interleaved(chunked.data() + cursor * 2, count, 2);
            cursor += count;
        }
        for (std::size_t index = 0; index < contiguous.size(); ++index) {
            assert(contiguous[index] == chunked[index]);
        }
    }

    {
        auto first_pass = test_signal(10000, 2);
        auto second_pass = first_pass;
        echo::audio::TransformVfxProcessor processor(
            enabled(echo::audio::TransformVfxCharacter::Ghost),
            kSampleRate,
            2
        );
        processor.process_interleaved(first_pass.data(), first_pass.size() / 2, 2);
        processor.reset();
        processor.process_interleaved(second_pass.data(), second_pass.size() / 2, 2);
        assert(first_pass == second_pass);
    }

    {
        auto live = test_signal(16000, 2);
        echo::audio::TransformVfxProcessor processor(
            enabled(echo::audio::TransformVfxCharacter::Robot),
            kSampleRate,
            2
        );
        processor.process_interleaved(live.data(), 7000, 2);
        processor.update(enabled(echo::audio::TransformVfxCharacter::Monster));
        processor.process_interleaved(live.data() + 7000 * 2, 9000, 2);
        for (float sample : live) {
            assert(std::isfinite(sample));
            assert(std::abs(sample) < 2.0F);
        }
        for (std::size_t frame = 7001; frame < 8500; ++frame) {
            assert(std::abs(live[frame * 2] - live[(frame - 1) * 2]) < 0.35F);
        }
    }

    {
        auto allocation_probe = test_signal(4096, 2);
        echo::audio::TransformVfxProcessor processor(
            enabled(echo::audio::TransformVfxCharacter::Giant),
            kSampleRate,
            2
        );
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        processor.update(enabled(echo::audio::TransformVfxCharacter::Tiny));
        processor.process_interleaved(allocation_probe.data(), allocation_probe.size() / 2, 2);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    return 0;
}
