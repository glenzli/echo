#include "echo/audio/tape_vfx_processor.hpp"

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

std::vector<float> fixture(std::size_t frames) {
    std::vector<float> samples(frames * kChannels, 0.0F);
    for (std::size_t frame = 0; frame < frames; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * kChannels] = 0.72F * std::sin(2.0F * 3.14159265F * 311.0F * time);
        samples[frame * kChannels + 1] = 0.58F * std::sin(2.0F * 3.14159265F * 719.0F * time);
    }
    return samples;
}

void assert_finite(const std::vector<float>& samples) {
    assert(std::all_of(samples.begin(), samples.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) < 4.0F;
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
        echo::audio::TapeVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(output == input);
        assert(processor.is_bypassed());
        assert(processor.latency_frames() == 0);
    }
    {
        const echo::audio::TapeVfxParameters parameters{
            .enabled = true,
            .mix_percent = 100,
            .saturation_percent = 55,
            .wow_flutter_percent = 65,
            .dropout_percent = 100,
        };
        echo::audio::TapeVfxProcessor processor(parameters, kSampleRate, kChannels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert_finite(output);
        assert(output != input);

        std::vector<float> silence(4096 * kChannels, 0.0F);
        processor.reset();
        processor.process_interleaved(silence.data(), silence.size() / kChannels, kChannels);
        assert(std::all_of(silence.begin(), silence.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }
    {
        echo::audio::TapeVfxProcessor processor({}, kSampleRate, kChannels);
        std::array<float, 1024> samples{};
        allocation_count.store(0, std::memory_order_relaxed);
        allocation_tracking.store(true, std::memory_order_relaxed);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        allocation_tracking.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }
    {
        bool rejected = false;
        try {
            echo::audio::TapeVfxParameters invalid{};
            invalid.mix_percent = 101;
            echo::audio::TapeVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }
}
