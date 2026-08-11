#include "echo/audio/de_plosive_processor.hpp"

#include <algorithm>
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

echo::audio::DePlosiveAdjustment enabled_adjustment() {
    return {
        .enabled = true,
        .frequency_hertz = 160,
        .sensitivity_percent = 75,
        .reduction_centibels = 1800,
        .release_millis = 140,
    };
}

std::vector<float> speech_with_plosive(std::size_t frame_count, std::size_t channels = 1) {
    std::vector<float> samples(frame_count * channels, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        const float voice = 0.055F * std::sin(2.0F * kPi * 900.0F * time);
        float plosive = 0.0F;
        if (frame >= 4000 && frame < 6400) {
            const float progress = static_cast<float>(frame - 4000) / 2400.0F;
            const float envelope = std::exp(-3.2F * progress);
            plosive = 0.82F * envelope * std::sin(2.0F * kPi * 85.0F * time);
        }
        for (std::size_t channel = 0; channel < channels; ++channel) {
            const float channel_scale = channel == 0 ? 1.0F : 0.55F;
            samples[frame * channels + channel] = voice + channel_scale * plosive;
        }
    }
    return samples;
}

double energy(const std::vector<float>& samples, std::size_t first, std::size_t end) {
    double sum = 0.0;
    for (std::size_t frame = first; frame < end; ++frame) {
        const double sample = samples[frame];
        sum += sample * sample;
    }
    return sum / static_cast<double>(end - first);
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
    static_assert(echo::audio::DePlosiveProcessor::latency_frames() == 0);
    {
        bool rejected = false;
        try {
            echo::audio::DePlosiveProcessor invalid({.frequency_hertz = 79}, kSampleRate, 1);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    const auto input = speech_with_plosive(12000);
    {
        auto bypassed = input;
        echo::audio::DePlosiveProcessor processor({}, kSampleRate, 1);
        processor.process_interleaved(bypassed.data(), bypassed.size(), 1);
        assert(bypassed == input);
    }
    auto processed = input;
    echo::audio::DePlosiveProcessor processor(enabled_adjustment(), kSampleRate, 1);
    processor.process_interleaved(processed.data(), processed.size(), 1);
    assert(energy(processed, 4200, 5600) < energy(input, 4200, 5600) * 0.70);
    assert(std::abs(energy(processed, 1000, 3000) - energy(input, 1000, 3000)) < 1.0E-7);
    assert(processor.attenuation_decibels() >= 0.0F);

    {
        auto contiguous = speech_with_plosive(15000, 2);
        auto chunked = contiguous;
        echo::audio::DePlosiveProcessor first(enabled_adjustment(), kSampleRate, 2);
        echo::audio::DePlosiveProcessor second(enabled_adjustment(), kSampleRate, 2);
        first.process_interleaved(contiguous.data(), contiguous.size() / 2, 2);
        std::size_t cursor = 0;
        for (const std::size_t chunk : {1U, 17U, 113U, 509U, 2048U}) {
            if (cursor >= chunked.size() / 2) {
                break;
            }
            const std::size_t frames = std::min(chunk, chunked.size() / 2 - cursor);
            second.process_interleaved(chunked.data() + cursor * 2, frames, 2);
            cursor += frames;
        }
        while (cursor < chunked.size() / 2) {
            const std::size_t frames = std::min<std::size_t>(137, chunked.size() / 2 - cursor);
            second.process_interleaved(chunked.data() + cursor * 2, frames, 2);
            cursor += frames;
        }
        for (std::size_t index = 0; index < contiguous.size(); ++index) {
            assert(std::abs(contiguous[index] - chunked[index]) < 1.0E-6F);
        }
    }

    {
        auto live = speech_with_plosive(24000);
        echo::audio::DePlosiveProcessor smooth(enabled_adjustment(), kSampleRate, 1);
        smooth.process_interleaved(live.data(), 8000, 1);
        smooth.update({});
        smooth.process_interleaved(live.data() + 8000, live.size() - 8000, 1);
        float maximum_step = 0.0F;
        for (std::size_t frame = 7801; frame < 9200; ++frame) {
            maximum_step = std::max(maximum_step, std::abs(live[frame] - live[frame - 1]));
        }
        assert(maximum_step < 0.05F);
    }

    {
        auto allocation_probe = speech_with_plosive(4096, 2);
        echo::audio::DePlosiveProcessor realtime(enabled_adjustment(), kSampleRate, 2);
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        realtime.process_interleaved(allocation_probe.data(), allocation_probe.size() / 2, 2);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    return 0;
}
