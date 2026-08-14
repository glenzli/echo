#include "echo/audio/freeze_vfx_processor.hpp"

#include <algorithm>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <limits>
#include <new>
#include <numeric>
#include <stdexcept>
#include <vector>

namespace {

std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocation_count = 0;

constexpr std::uint32_t kSampleRate = 48000;
constexpr std::size_t kLatency = echo::audio::FreezeVfxProcessor::latency_frames();
constexpr float kPi = 3.14159265358979323846F;

echo::audio::FreezeVfxAdjustment enabled() {
    return {.enabled = true, .mix_percent = 100};
}

std::vector<float> sine(std::size_t frames, std::size_t channels) {
    std::vector<float> result(frames * channels, 0.0F);
    for (std::size_t frame = 0; frame < frames; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        result[frame * channels] = 0.35F * std::sin(2.0F * kPi * 437.0F * time);
        if (channels == 2) {
            result[frame * channels + 1] = 0.23F * std::sin(2.0F * kPi * 683.0F * time + 0.37F);
        }
    }
    return result;
}

void process_blocked(
    echo::audio::FreezeVfxProcessor& processor,
    float* samples,
    std::size_t frames,
    std::size_t channels
) {
    std::size_t cursor = 0;
    while (cursor < frames) {
        const std::size_t count = std::min<std::size_t>(1 + cursor % 257, frames - cursor);
        processor.process_interleaved(samples + cursor * channels, count, channels);
        cursor += count;
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
    static_assert(kLatency == 4096);

    {
        std::vector<float> impulse(kLatency * 2 + 1, 0.0F);
        impulse[0] = 1.0F;
        echo::audio::FreezeVfxProcessor processor({}, kSampleRate, 1);
        processor.process_interleaved(impulse.data(), impulse.size(), 1);
        for (std::size_t frame = 0; frame < kLatency; ++frame) {
            assert(impulse[frame] == 0.0F);
        }
        assert(impulse[kLatency] == 1.0F);
        for (std::size_t frame = kLatency + 1; frame < impulse.size(); ++frame) {
            assert(impulse[frame] == 0.0F);
        }
    }

    {
        echo::audio::FreezeVfxProcessor processor(enabled(), kSampleRate, 1);
        assert(!processor.request_capture());
        std::vector<float> pre_roll(kLatency, 0.0F);
        processor.process_interleaved(pre_roll.data(), pre_roll.size(), 1);
        assert(processor.request_capture());
    }

    {
        std::vector<float> silence(kLatency * 4, 0.0F);
        echo::audio::FreezeVfxProcessor processor(enabled(), kSampleRate, 2);
        processor.process_interleaved(silence.data(), kLatency, 2);
        assert(processor.request_capture());
        processor
            .process_interleaved(silence.data() + kLatency * 2, silence.size() / 2 - kLatency, 2);
        assert(processor.has_capture());
        assert(std::all_of(silence.begin(), silence.end(), [](float value) {
            return value == 0.0F;
        }));
    }

    {
        const std::size_t prefix_frames = kLatency * 2;
        const std::size_t suffix_frames = kLatency * 4;
        auto prefix = sine(prefix_frames, 1);
        std::vector<float> suffix(suffix_frames, 0.0F);
        echo::audio::FreezeVfxProcessor processor(enabled(), kSampleRate, 1);
        processor.process_interleaved(prefix.data(), prefix.size(), 1);
        assert(processor.request_capture());
        processor.process_interleaved(suffix.data(), suffix.size(), 1);
        assert(processor.has_capture());
        const auto begin = suffix.begin() + static_cast<std::ptrdiff_t>(kLatency + 2048);
        const float peak =
            std::accumulate(begin, suffix.end(), 0.0F, [](float current, float value) {
                return std::max(current, std::abs(value));
            });
        assert(peak > 0.03F);
    }

    {
        constexpr std::size_t prefix_frames = kLatency * 2 + 333;
        constexpr std::size_t suffix_frames = kLatency * 3;
        auto contiguous_prefix = sine(prefix_frames, 2);
        auto blocked_prefix = contiguous_prefix;
        std::vector<float> contiguous_suffix(suffix_frames * 2, 0.0F);
        auto blocked_suffix = contiguous_suffix;
        echo::audio::FreezeVfxProcessor contiguous(enabled(), kSampleRate, 2);
        echo::audio::FreezeVfxProcessor blocked(enabled(), kSampleRate, 2);
        contiguous.process_interleaved(contiguous_prefix.data(), prefix_frames, 2);
        process_blocked(blocked, blocked_prefix.data(), prefix_frames, 2);
        assert(contiguous.request_capture());
        assert(blocked.request_capture());
        contiguous.process_interleaved(contiguous_suffix.data(), suffix_frames, 2);
        process_blocked(blocked, blocked_suffix.data(), suffix_frames, 2);
        assert(contiguous_prefix == blocked_prefix);
        assert(contiguous_suffix == blocked_suffix);
    }

    {
        auto program = sine(kLatency * 3, 2);
        echo::audio::FreezeVfxProcessor processor(enabled(), kSampleRate, 2);
        processor.process_interleaved(program.data(), kLatency * 2, 2);
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        assert(processor.request_capture());
        processor.update({.enabled = true, .mix_percent = 65});
        processor.process_interleaved(program.data() + kLatency * 4, kLatency, 2);
        processor.reset();
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
        assert(!processor.has_capture());
        assert(!processor.capture_pending());
    }

    {
        bool rejected = false;
        try {
            [[maybe_unused]] echo::audio::FreezeVfxProcessor invalid(
                {.enabled = true, .mix_percent = 101},
                kSampleRate,
                2
            );
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
        rejected = false;
        try {
            [[maybe_unused]] echo::audio::FreezeVfxProcessor invalid({}, 44100, 2);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    {
        std::vector<float> hostile(kLatency * 2, 0.0F);
        hostile[0] = std::numeric_limits<float>::quiet_NaN();
        hostile[1] = std::numeric_limits<float>::infinity();
        echo::audio::FreezeVfxProcessor processor({}, kSampleRate, 1);
        processor.process_interleaved(hostile.data(), hostile.size(), 1);
        assert(std::all_of(hostile.begin(), hostile.end(), [](float value) {
            return std::isfinite(value);
        }));
    }

    return 0;
}
