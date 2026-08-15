#include "echo/audio/beat_repeat_vfx_processor.hpp"

#include <algorithm>
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
std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocations = 0;

std::vector<float> input(std::size_t frames) {
    std::vector<float> values(frames * kChannels);
    for (std::size_t frame = 0; frame < frames; ++frame) {
        values[frame * kChannels] = static_cast<float>(frame % 997) / 997.0F;
        values[frame * kChannels + 1] = static_cast<float>((frame * 7) % 991) / 991.0F;
    }
    return values;
}

} // namespace

void* operator new(std::size_t size) {
    if (track_allocations.load(std::memory_order_relaxed)) {
        allocations.fetch_add(1, std::memory_order_relaxed);
    }
    if (void* memory = std::malloc(size)) {
        return memory;
    }
    throw std::bad_alloc();
}
void operator delete(void* memory) noexcept {
    std::free(memory);
}
void operator delete(void* memory, std::size_t) noexcept {
    std::free(memory);
}

int main() {
    {
        echo::audio::BeatRepeatVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = input(100000);
        const auto expected = output;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(processor.is_bypassed());
        assert(processor.latency_frames() == 96000);
        assert(
            std::all_of(
                output.begin(),
                output.begin()
                    + static_cast<std::ptrdiff_t>(processor.latency_frames() * kChannels),
                [](float value) { return value == 0.0F; }
            )
        );
        assert(
            std::equal(
                output.begin()
                    + static_cast<std::ptrdiff_t>(processor.latency_frames() * kChannels),
                output.end(),
                expected.begin()
            )
        );
    }
    {
        constexpr std::size_t kSliceFrames = 1440;
        const auto source = input(120000);
        auto forward = source;
        auto reversed = source;
        echo::audio::BeatRepeatVfxProcessor forward_processor(
            {.enabled = true,
             .mix_percent = 100,
             .slice_millis = 30,
             .repeat_count = 3,
             .reverse = false},
            kSampleRate,
            kChannels
        );
        echo::audio::BeatRepeatVfxProcessor reverse_processor(
            {.enabled = true,
             .mix_percent = 100,
             .slice_millis = 30,
             .repeat_count = 3,
             .reverse = true},
            kSampleRate,
            kChannels
        );
        forward_processor
            .process_interleaved(forward.data(), forward.size() / kChannels, kChannels);
        reverse_processor
            .process_interleaved(reversed.data(), reversed.size() / kChannels, kChannels);
        assert(std::all_of(forward.begin(), forward.end(), [](float value) {
            return std::isfinite(value);
        }));
        assert(forward != reversed);
        const std::size_t audible_start = forward_processor.latency_frames() * kChannels;
        assert(forward[audible_start] == source[0]);
        assert(forward[audible_start + kSliceFrames * kChannels] == source[0]);
        assert(reversed[audible_start] == source[(kSliceFrames - 1) * kChannels]);
    }
    {
        echo::audio::BeatRepeatVfxProcessor processor({}, kSampleRate, kChannels);
        auto samples = input(1024);
        allocations.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocations.load(std::memory_order_relaxed) == 0);
    }
    {
        bool rejected = false;
        try {
            echo::audio::BeatRepeatVfxProcessor processor(
                {.enabled = true,
                 .mix_percent = 100,
                 .slice_millis = 501,
                 .repeat_count = 2,
                 .reverse = false},
                kSampleRate,
                kChannels
            );
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }
}
