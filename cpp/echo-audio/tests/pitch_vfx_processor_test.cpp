#include "echo/audio/pitch_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <iterator>
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
        samples[frame * kChannels] = 0.72F * std::sin(2.0F * 3.14159265F * 227.0F * time);
        samples[frame * kChannels + 1] = 0.48F * std::sin(2.0F * 3.14159265F * 521.0F * time);
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
    const auto input = fixture(8192);
    {
        echo::audio::PitchVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(processor.is_bypassed());
        assert(processor.latency_frames() == 576);
        assert(
            std::all_of(
                output.begin(),
                std::next(
                    output.begin(),
                    static_cast<std::ptrdiff_t>(2 * processor.latency_frames())
                ),
                [](float sample) { return sample == 0.0F; }
            )
        );
    }
    {
        const echo::audio::PitchVfxParameters parameters{
            .enabled = true,
            .mix_percent = 100,
            .pitch_semitones = 7,
            .harmony_enabled = true,
            .harmony_semitones = -5,
            .harmony_mix_percent = 42,
            .formant_colour_semitones = 4,
        };
        echo::audio::PitchVfxProcessor processor(parameters, kSampleRate, kChannels);
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
        echo::audio::PitchVfxProcessor processor({}, kSampleRate, kChannels);
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
            echo::audio::PitchVfxParameters invalid{};
            invalid.pitch_semitones = 13;
            echo::audio::PitchVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }
}
