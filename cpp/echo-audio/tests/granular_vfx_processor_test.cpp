#include "echo/audio/granular_vfx_processor.hpp"

#include <algorithm>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <limits>
#include <new>
#include <stdexcept>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;
constexpr std::size_t kChannels = 2;
constexpr float kPi = 3.14159265358979323846F;
std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocation_count = 0;

echo::audio::GranularVfxAdjustment enabled(std::uint32_t seed = 0x12345678U) {
    echo::audio::GranularVfxAdjustment adjustment;
    adjustment.enabled = true;
    adjustment.mix_percent = 70;
    adjustment.grain_millis = 80;
    adjustment.density_tenths_hertz = 160;
    adjustment.lookback_millis = 180;
    adjustment.scatter_millis = 90;
    adjustment.pitch_cents = 300;
    adjustment.stereo_spread_percent = 70;
    adjustment.random_seed = seed;
    return adjustment;
}

std::vector<float> program(std::size_t frame_count, std::size_t channel_count = kChannels) {
    std::vector<float> samples(frame_count * channel_count, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * channel_count] = 0.52F * std::sin(2.0F * kPi * 317.0F * time)
                                         + 0.17F * std::sin(2.0F * kPi * 1903.0F * time);
        if (channel_count == 2) {
            samples[frame * channel_count + 1] =
                0.43F * std::sin(2.0F * kPi * 479.0F * time + 0.27F)
                - 0.13F * std::sin(2.0F * kPi * 2741.0F * time);
        }
    }
    return samples;
}

std::vector<float> process(
    echo::audio::GranularVfxAdjustment adjustment,
    const std::vector<float>& input,
    std::size_t channel_count,
    std::size_t block_frames,
    std::uint64_t absolute_frame = 0
) {
    echo::audio::GranularVfxProcessor processor(adjustment, kSampleRate, channel_count);
    processor.reset(absolute_frame);
    auto output = input;
    const std::size_t frames = output.size() / channel_count;
    for (std::size_t offset = 0; offset < frames;) {
        const std::size_t count = std::min(block_frames, frames - offset);
        processor.process_interleaved(output.data() + offset * channel_count, count, channel_count);
        offset += count;
    }
    return output;
}

double difference_energy(const std::vector<float>& left, const std::vector<float>& right) {
    assert(left.size() == right.size());
    double energy = 0.0;
    for (std::size_t index = 0; index < left.size(); ++index) {
        const double difference = static_cast<double>(left[index] - right[index]);
        energy += difference * difference;
    }
    return energy / static_cast<double>(left.size());
}

bool throws_invalid(const echo::audio::GranularVfxAdjustment& adjustment) {
    try {
        echo::audio::GranularVfxProcessor processor(adjustment, kSampleRate, kChannels);
    } catch (const std::invalid_argument&) {
        return true;
    }
    return false;
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
    using echo::audio::GranularVfxAdjustment;
    using echo::audio::GranularVfxProcessor;

    static_assert(GranularVfxProcessor::latency_frames() == 0);
    const auto source = program(kSampleRate * 2U);

    {
        GranularVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = source;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(processor.is_bypassed());
        assert(output == source);
    }

    {
        std::vector<float> silence(kSampleRate * kChannels, 0.0F);
        const auto output = process(enabled(), silence, kChannels, 127);
        assert(std::all_of(output.begin(), output.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }

    const auto contiguous = process(enabled(), source, kChannels, source.size() / kChannels);
    assert(difference_energy(contiguous, source) > 1.0E-4);
    for (const std::size_t block_frames : {1U, 17U, 257U, 4096U}) {
        assert(process(enabled(), source, kChannels, block_frames) == contiguous);
    }

    {
        const auto same_seed = process(enabled(), source, kChannels, 113);
        const auto other_seed = process(enabled(0x87654321U), source, kChannels, 113);
        const auto other_position = process(enabled(), source, kChannels, 113, 919U);
        assert(same_seed == contiguous);
        assert(difference_energy(same_seed, other_seed) > 1.0E-6);
        assert(difference_energy(same_seed, other_position) > 1.0E-6);
    }

    {
        GranularVfxAdjustment adjustment = enabled();
        adjustment.enabled = false;
        GranularVfxProcessor processor(adjustment, kSampleRate, 1);
        auto captured = program(kSampleRate, 1);
        const auto original = captured;
        processor.process_interleaved(captured.data(), captured.size(), 1);
        assert(captured == original);
        adjustment.enabled = true;
        processor.update(adjustment);
        std::vector<float> silence(kSampleRate / 4U, 0.0F);
        processor.process_interleaved(silence.data(), silence.size(), 1);
        assert(std::any_of(silence.begin(), silence.end(), [](float sample) {
            return sample != 0.0F;
        }));
    }

    {
        auto left_only = source;
        for (std::size_t frame = 0; frame < left_only.size() / kChannels; ++frame) {
            left_only[frame * kChannels + 1] = 0.0F;
        }
        const auto output = process(enabled(), left_only, kChannels, 211);
        for (std::size_t frame = 0; frame < output.size() / kChannels; ++frame) {
            assert(output[frame * kChannels + 1] == 0.0F);
        }
    }

    {
        auto mono = program(kSampleRate * 2U, 1);
        const auto output = process(enabled(), mono, 1, 137);
        assert(difference_energy(output, mono) > 1.0E-4);
    }

    {
        const auto adjustment = enabled();
        GranularVfxProcessor reset_processor(adjustment, kSampleRate, kChannels);
        GranularVfxProcessor fresh_processor(adjustment, kSampleRate, kChannels);
        auto warmup = program(777);
        reset_processor.process_interleaved(warmup.data(), warmup.size() / kChannels, kChannels);
        reset_processor.reset(1234U);
        fresh_processor.reset(1234U);
        auto reset_output = source;
        auto fresh_output = source;
        reset_processor
            .process_interleaved(reset_output.data(), reset_output.size() / kChannels, kChannels);
        fresh_processor
            .process_interleaved(fresh_output.data(), fresh_output.size() / kChannels, kChannels);
        assert(reset_output == fresh_output);
    }

    {
        // A linear processor-input timeline can be rebuilt deterministically.
        // Playback seeks intentionally choose a fresh zero timeline instead of
        // claiming parity across collapsed source edits and authored gaps.
        auto disabled = enabled(0xA5A5A5A5U);
        disabled.enabled = false;
        constexpr std::uint64_t kRebuildSourceFrame = 123457U;
        GranularVfxProcessor continuous(disabled, kSampleRate, kChannels);
        GranularVfxProcessor rebuilt(disabled, kSampleRate, kChannels);
        std::vector<float> skipped(kRebuildSourceFrame * kChannels, 0.0F);
        continuous.process_interleaved(skipped.data(), kRebuildSourceFrame, kChannels);
        rebuilt.reset(kRebuildSourceFrame);

        auto active = disabled;
        active.enabled = true;
        continuous.update(active);
        rebuilt.update(active);
        std::vector<float> history(kSampleRate * 2U * kChannels, 0.0F);
        auto rebuilt_history = history;
        continuous.process_interleaved(history.data(), history.size() / kChannels, kChannels);
        rebuilt.process_interleaved(
            rebuilt_history.data(),
            rebuilt_history.size() / kChannels,
            kChannels
        );
        auto continuous_output = source;
        auto rebuilt_output = source;
        continuous.process_interleaved(
            continuous_output.data(),
            continuous_output.size() / kChannels,
            kChannels
        );
        rebuilt.process_interleaved(
            rebuilt_output.data(),
            rebuilt_output.size() / kChannels,
            kChannels
        );
        assert(continuous_output == rebuilt_output);
    }

    {
        auto hostile = std::vector<float>(2048 * kChannels, 0.0F);
        hostile[0] = std::numeric_limits<float>::quiet_NaN();
        hostile[1] = std::numeric_limits<float>::infinity();
        hostile[2] = -std::numeric_limits<float>::infinity();
        hostile[3] = std::numeric_limits<float>::max();
        GranularVfxProcessor processor(enabled(), kSampleRate, kChannels);
        processor.process_interleaved(hostile.data(), hostile.size() / kChannels, kChannels);
        assert(std::all_of(hostile.begin(), hostile.end(), [](float sample) {
            return std::isfinite(sample) && std::abs(sample) <= 16.0F;
        }));
    }

    {
        auto invalid = enabled();
        invalid.mix_percent = 101;
        assert(throws_invalid(invalid));
        invalid = enabled();
        invalid.grain_millis = 19;
        assert(throws_invalid(invalid));
        invalid = enabled();
        invalid.density_tenths_hertz = 401;
        assert(throws_invalid(invalid));
        invalid = enabled();
        invalid.lookback_millis = 1500;
        invalid.scatter_millis = 750;
        invalid.grain_millis = 250;
        invalid.pitch_cents = 1200;
        assert(throws_invalid(invalid));
        invalid = enabled();
        invalid.lookback_millis = 1500;
        invalid.scatter_millis = 300;
        invalid.grain_millis = 250;
        invalid.pitch_cents = -1200;
        assert(throws_invalid(invalid));
        bool threw = false;
        try {
            GranularVfxProcessor processor(enabled(), 44100, kChannels);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);
        threw = false;
        try {
            GranularVfxProcessor processor(enabled(), kSampleRate, 3);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);
    }

    {
        auto adjustment = enabled();
        GranularVfxProcessor processor(adjustment, kSampleRate, kChannels);
        auto samples = source;
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        adjustment.mix_percent = 35;
        adjustment.pitch_cents = -500;
        adjustment.random_seed = 7;
        processor.update(adjustment);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        processor.reset(77U);
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    return 0;
}
