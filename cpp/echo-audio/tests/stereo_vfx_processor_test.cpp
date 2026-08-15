#include "echo/audio/stereo_vfx_processor.hpp"

#include <array>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <new>
#include <stdexcept>

namespace {

std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocations = 0;

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
    constexpr std::uint32_t sample_rate = 48000;
    constexpr std::size_t channels = 2;
    const std::array<float, 8> input{0.8F, 0.2F, -0.4F, 0.3F, 0.6F, -0.2F, 0.1F, -0.7F};
    {
        echo::audio::StereoVfxProcessor processor({}, sample_rate, channels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / channels, channels);
        assert(output == input);
        assert(processor.is_bypassed());
        assert(processor.latency_frames() == 0);
    }
    {
        echo::audio::StereoVfxProcessor processor(
            {.enabled = true, .mix_percent = 100, .width_percent = 150, .pan_percent = 0},
            sample_rate,
            channels
        );
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / channels, channels);
        assert(output != input);
        assert(std::isfinite(output[0]) && std::isfinite(output[1]));
    }
    {
        echo::audio::StereoVfxProcessor processor(
            {.enabled = true, .mix_percent = 100, .width_percent = 100, .pan_percent = 70},
            sample_rate,
            channels
        );
        std::array<float, 2> output{0.5F, 0.5F};
        processor.process_interleaved(output.data(), 1, channels);
        assert(std::abs(output[1]) > std::abs(output[0]));
    }
    {
        echo::audio::StereoVfxProcessor processor({}, sample_rate, channels);
        std::array<float, 256> samples{};
        allocations.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        processor.process_interleaved(samples.data(), samples.size() / channels, channels);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocations.load(std::memory_order_relaxed) == 0);
    }
    {
        bool rejected = false;
        try {
            echo::audio::StereoVfxProcessor processor(
                {.enabled = true, .mix_percent = 100, .width_percent = 201, .pan_percent = 0},
                sample_rate,
                channels
            );
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }
}
