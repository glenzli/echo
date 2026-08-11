#include "echo/audio/delay_vfx_processor.hpp"

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

echo::audio::DelayVfxAdjustment slapback() {
    echo::audio::DelayVfxAdjustment value;
    value.enabled = true;
    value.character = echo::audio::DelayVfxCharacter::Slapback;
    value.slapback.delay_millis = 90;
    value.slapback.mix_percent = 100;
    value.slapback.high_cut_hertz = 20000;
    return value;
}

echo::audio::DelayVfxAdjustment echo_adjustment() {
    echo::audio::DelayVfxAdjustment value;
    value.enabled = true;
    value.character = echo::audio::DelayVfxCharacter::Echo;
    value.echo.delay_millis = 100;
    value.echo.feedback_percent = 50;
    value.echo.mix_percent = 100;
    value.echo.high_cut_hertz = 20000;
    value.echo.stereo_crossfeed_percent = 100;
    return value;
}

std::vector<float> stereo_program(std::size_t frame_count) {
    std::vector<float> samples(frame_count * 2, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * 2] = 0.31F * std::sin(2.0F * kPi * 431.0F * time);
        samples[frame * 2 + 1] = 0.23F * std::sin(2.0F * kPi * 719.0F * time + 0.4F);
    }
    return samples;
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
    static_assert(echo::audio::DelayVfxProcessor::latency_frames() == 0);
    {
        auto input = stereo_program(12000);
        auto bypassed = input;
        echo::audio::DelayVfxProcessor processor({}, kSampleRate, 2);
        assert(processor.is_bypassed());
        processor.process_interleaved(bypassed.data(), bypassed.size() / 2, 2);
        assert(bypassed == input);
    }

    {
        const std::size_t delay_frames = 90 * kSampleRate / 1000;
        std::vector<float> impulse(delay_frames * 3, 0.0F);
        impulse[0] = 1.0F;
        echo::audio::DelayVfxProcessor processor(slapback(), kSampleRate, 1);
        processor.process_interleaved(impulse.data(), impulse.size(), 1);
        for (std::size_t frame = 0; frame < delay_frames; ++frame) {
            assert(impulse[frame] == 0.0F);
        }
        assert(impulse[delay_frames] > 0.85F);
        for (std::size_t frame = delay_frames + 256; frame < impulse.size(); ++frame) {
            assert(std::abs(impulse[frame]) < 1.0E-7F);
        }
    }

    {
        const std::size_t delay_frames = 100 * kSampleRate / 1000;
        std::vector<float> impulse((delay_frames * 3 + 1) * 2, 0.0F);
        impulse[0] = 1.0F;
        echo::audio::DelayVfxProcessor processor(echo_adjustment(), kSampleRate, 2);
        processor.process_interleaved(impulse.data(), impulse.size() / 2, 2);
        assert(impulse[delay_frames * 2] > 0.85F);
        assert(std::abs(impulse[delay_frames * 2 + 1]) < 1.0E-7F);
        assert(impulse[delay_frames * 4 + 1] > 0.35F);
        assert(std::abs(impulse[delay_frames * 4]) < 1.0E-6F);
        assert(impulse[delay_frames * 6] > 0.14F);
        for (const float sample : impulse) {
            assert(std::isfinite(sample));
            assert(std::abs(sample) < 2.0F);
        }
    }

    {
        auto contiguous = stereo_program(20000);
        auto blocked = contiguous;
        echo::audio::DelayVfxProcessor first(echo_adjustment(), kSampleRate, 2);
        echo::audio::DelayVfxProcessor second(echo_adjustment(), kSampleRate, 2);
        first.process_interleaved(contiguous.data(), contiguous.size() / 2, 2);
        std::size_t cursor = 0;
        while (cursor < blocked.size() / 2) {
            const std::size_t frames =
                std::min<std::size_t>(137 + cursor % 509, blocked.size() / 2 - cursor);
            second.process_interleaved(blocked.data() + cursor * 2, frames, 2);
            cursor += frames;
        }
        assert(contiguous == blocked);
    }

    {
        std::vector<float> tail(kSampleRate, 0.0F);
        tail[0] = 1.0F;
        echo::audio::DelayVfxProcessor processor(echo_adjustment(), kSampleRate, 1);
        processor.process_interleaved(tail.data(), tail.size(), 1);
        processor.reset();
        std::fill(tail.begin(), tail.end(), 0.0F);
        processor.process_interleaved(tail.data(), tail.size(), 1);
        assert(std::all_of(tail.begin(), tail.end(), [](float sample) { return sample == 0.0F; }));
    }

    {
        auto probe = stereo_program(8192);
        echo::audio::DelayVfxProcessor processor(slapback(), kSampleRate, 2);
        auto target = echo_adjustment();
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        processor.update(target);
        processor.process_interleaved(probe.data(), probe.size() / 2, 2);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
        for (const float sample : probe) {
            assert(std::isfinite(sample));
        }
    }

    {
        bool rejected = false;
        auto invalid = echo_adjustment();
        invalid.echo.feedback_percent = 91;
        try {
            echo::audio::DelayVfxProcessor processor(invalid, kSampleRate, 2);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    return 0;
}
