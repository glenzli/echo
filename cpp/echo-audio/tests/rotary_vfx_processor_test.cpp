#include "echo/audio/rotary_vfx_processor.hpp"

#include <algorithm>
#include <atomic>
#include <cassert>
#include <chrono>
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
std::atomic<bool> allocation_tracking = false;
std::atomic<std::size_t> allocation_count = 0;

echo::audio::RotaryVfxAdjustment enabled(echo::audio::RotaryVfxSpeed speed) {
    return {
        .speed = speed,
        .enabled = true,
        .mix_percent = 100,
        .motion_percent = 80,
        .stereo_width_percent = 90,
    };
}

std::vector<float> fixture(std::size_t frame_count, std::size_t channel_count = kChannels) {
    std::vector<float> samples(frame_count * channel_count, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * channel_count] = 0.55F * std::sin(2.0F * kPi * 311.0F * time)
                                         + 0.2F * std::sin(2.0F * kPi * 2197.0F * time);
        if (channel_count == 2) {
            samples[frame * channel_count + 1] =
                0.43F * std::sin(2.0F * kPi * 487.0F * time + 0.31F)
                - 0.18F * std::sin(2.0F * kPi * 3479.0F * time);
        }
    }
    return samples;
}

std::vector<float> pure_tone(std::size_t frame_count, float frequency_hertz) {
    std::vector<float> samples(frame_count, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame] = 0.5F * std::sin(2.0F * kPi * frequency_hertz * time);
    }
    return samples;
}

std::vector<float> process(
    echo::audio::RotaryVfxAdjustment adjustment,
    const std::vector<float>& input,
    std::size_t channel_count,
    std::size_t block_frames
) {
    echo::audio::RotaryVfxProcessor processor(adjustment, kSampleRate, channel_count);
    auto output = input;
    const std::size_t frame_count = output.size() / channel_count;
    std::size_t offset = 0;
    while (offset < frame_count) {
        const std::size_t count = std::min(block_frames, frame_count - offset);
        processor.process_interleaved(output.data() + offset * channel_count, count, channel_count);
        offset += count;
    }
    return output;
}

void assert_near(const std::vector<float>& actual, const std::vector<float>& expected) {
    assert(actual.size() == expected.size());
    for (std::size_t index = 0; index < actual.size(); ++index) {
        assert(std::abs(actual[index] - expected[index]) < 1.0E-6F);
    }
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

std::vector<float>
window_rms(const std::vector<float>& samples, std::size_t first_frame, std::size_t window_frames) {
    std::vector<float> envelope;
    for (std::size_t offset = first_frame; offset + window_frames <= samples.size();
         offset += window_frames) {
        double energy = 0.0;
        for (std::size_t index = 0; index < window_frames; ++index) {
            const double sample = static_cast<double>(samples[offset + index]);
            energy += sample * sample;
        }
        envelope.push_back(
            static_cast<float>(std::sqrt(energy / static_cast<double>(window_frames)))
        );
    }
    return envelope;
}

std::size_t local_peak_count(const std::vector<float>& values) {
    std::size_t peaks = 0;
    for (std::size_t index = 1; index + 1 < values.size(); ++index) {
        if (values[index] > values[index - 1] && values[index] >= values[index + 1]) {
            ++peaks;
        }
    }
    return peaks;
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
    using echo::audio::RotaryVfxProcessor;
    using echo::audio::RotaryVfxSpeed;

    static_assert(RotaryVfxProcessor::latency_frames() == 0);
    const auto input = fixture(16384);

    {
        RotaryVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(processor.is_bypassed());
        assert(output == input);
        std::vector<float> over_range{20.0F, -24.0F};
        processor.process_interleaved(over_range.data(), 1, kChannels);
        assert((over_range == std::vector<float>{20.0F, -24.0F}));
    }

    {
        auto adjustment = enabled(RotaryVfxSpeed::Fast);
        adjustment.mix_percent = 0;
        assert(process(adjustment, input, kChannels, 257) == input);
        const std::vector<float> over_range{20.0F, -24.0F};
        assert(process(adjustment, over_range, kChannels, 1) == over_range);
        adjustment.mix_percent = 100;
        adjustment.motion_percent = 0;
        assert(process(adjustment, input, kChannels, 257) == input);
    }

    const auto slow = process(enabled(RotaryVfxSpeed::Slow), input, kChannels, 257);
    const auto fast = process(enabled(RotaryVfxSpeed::Fast), input, kChannels, 257);
    assert(difference_energy(slow, input) > 1.0E-5);
    assert(difference_energy(fast, input) > 1.0E-5);
    assert(difference_energy(slow, fast) > 1.0E-5);
    assert(std::all_of(fast.begin(), fast.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) <= 16.0F;
    }));

    {
        const auto horn_tone = pure_tone(kSampleRate * 6U, 3000.0F);
        const auto slow_horn = process(enabled(RotaryVfxSpeed::Slow), horn_tone, 1, 257);
        const auto fast_horn = process(enabled(RotaryVfxSpeed::Fast), horn_tone, 1, 257);
        const std::size_t slow_peaks = local_peak_count(window_rms(slow_horn, kSampleRate, 512));
        const std::size_t fast_peaks = local_peak_count(window_rms(fast_horn, kSampleRate, 512));
        assert(slow_peaks >= 3);
        assert(fast_peaks > slow_peaks * 3);
    }

    for (const std::size_t block_frames : {1U, 13U, 4096U}) {
        assert_near(fast, process(enabled(RotaryVfxSpeed::Fast), input, kChannels, block_frames));
    }

    {
        const auto mono = fixture(8192, 1);
        const auto mono_output = process(enabled(RotaryVfxSpeed::Slow), mono, 1, 137);
        assert(difference_energy(mono_output, mono) > 1.0E-5);
    }

    {
        auto left_only = fixture(8192);
        for (std::size_t frame = 0; frame < left_only.size() / kChannels; ++frame) {
            left_only[frame * kChannels + 1] = 0.0F;
        }
        const auto output = process(enabled(RotaryVfxSpeed::Fast), left_only, kChannels, 257);
        for (std::size_t frame = 0; frame < output.size() / kChannels; ++frame) {
            assert(output[frame * kChannels + 1] == 0.0F);
        }
    }

    {
        std::vector<float> silence(8192 * kChannels, 0.0F);
        const auto output = process(enabled(RotaryVfxSpeed::Fast), silence, kChannels, 31);
        assert(std::all_of(output.begin(), output.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }

    {
        std::vector<float> hostile(128 * kChannels, 0.0F);
        hostile[0] = std::numeric_limits<float>::quiet_NaN();
        hostile[1] = std::numeric_limits<float>::infinity();
        hostile[2] = -std::numeric_limits<float>::infinity();
        hostile[3] = std::numeric_limits<float>::max();
        hostile[4] = -std::numeric_limits<float>::max();
        RotaryVfxProcessor processor(enabled(RotaryVfxSpeed::Fast), kSampleRate, kChannels);
        processor.process_interleaved(hostile.data(), hostile.size() / kChannels, kChannels);
        assert(std::all_of(hostile.begin(), hostile.end(), [](float sample) {
            return std::isfinite(sample) && std::abs(sample) <= 16.0F;
        }));
    }

    {
        const auto adjustment = enabled(RotaryVfxSpeed::Slow);
        RotaryVfxProcessor used(adjustment, kSampleRate, kChannels);
        RotaryVfxProcessor fresh(adjustment, kSampleRate, kChannels);
        auto discarded = fixture(777);
        used.process_interleaved(discarded.data(), discarded.size() / kChannels, kChannels);
        used.reset();
        auto reset_output = input;
        auto fresh_output = input;
        used.process_interleaved(reset_output.data(), reset_output.size() / kChannels, kChannels);
        fresh.process_interleaved(fresh_output.data(), fresh_output.size() / kChannels, kChannels);
        assert_near(reset_output, fresh_output);
    }

    {
        auto adjustment = enabled(RotaryVfxSpeed::Slow);
        RotaryVfxProcessor processor(adjustment, kSampleRate, kChannels);
        auto warmup = fixture(kSampleRate * 2U);
        processor.process_interleaved(warmup.data(), warmup.size() / kChannels, kChannels);
        adjustment.speed = RotaryVfxSpeed::Fast;
        adjustment.motion_percent = 100;
        processor.update(adjustment);
        assert(processor.adjustment().speed == RotaryVfxSpeed::Fast);
        auto transition = fixture(kSampleRate * 2U);
        processor.process_interleaved(transition.data(), transition.size() / kChannels, kChannels);
        adjustment.speed = RotaryVfxSpeed::Brake;
        processor.update(adjustment);
        auto braking = fixture(kSampleRate * 3U);
        processor.process_interleaved(braking.data(), braking.size() / kChannels, kChannels);
        assert(std::all_of(braking.begin(), braking.end(), [](float sample) {
            return std::isfinite(sample) && std::abs(sample) <= 16.0F;
        }));
        float maximum_jump = 0.0F;
        for (std::size_t frame = 1; frame < braking.size() / kChannels; ++frame) {
            maximum_jump = std::max(
                maximum_jump,
                std::abs(braking[frame * kChannels] - braking[(frame - 1) * kChannels])
            );
        }
        assert(maximum_jump < 0.5F);
    }

    {
        auto adjustment = enabled(RotaryVfxSpeed::Slow);
        RotaryVfxProcessor horn(adjustment, kSampleRate, 1);
        RotaryVfxProcessor drum(adjustment, kSampleRate, 1);
        auto horn_warmup = pure_tone(kSampleRate * 2U, 3000.0F);
        auto drum_warmup = pure_tone(kSampleRate * 2U, 187.5F);
        horn.process_interleaved(horn_warmup.data(), horn_warmup.size(), 1);
        drum.process_interleaved(drum_warmup.data(), drum_warmup.size(), 1);
        adjustment.speed = RotaryVfxSpeed::Fast;
        horn.update(adjustment);
        drum.update(adjustment);
        auto horn_acceleration = pure_tone(kSampleRate * 4U, 3000.0F);
        auto drum_acceleration = pure_tone(kSampleRate * 4U, 187.5F);
        horn.process_interleaved(horn_acceleration.data(), horn_acceleration.size(), 1);
        drum.process_interleaved(drum_acceleration.data(), drum_acceleration.size(), 1);
        const auto horn_peaks = local_peak_count(window_rms(horn_acceleration, 0, 512));
        const auto drum_peaks = local_peak_count(window_rms(drum_acceleration, 0, 512));
        assert(horn_peaks > drum_peaks);

        adjustment.speed = RotaryVfxSpeed::Brake;
        horn.update(adjustment);
        drum.update(adjustment);
        auto stopped_horn = pure_tone(kSampleRate * 8U, 3000.0F);
        auto stopped_drum = pure_tone(kSampleRate * 8U, 187.5F);
        horn.process_interleaved(stopped_horn.data(), stopped_horn.size(), 1);
        drum.process_interleaved(stopped_drum.data(), stopped_drum.size(), 1);
        for (const auto* stopped : {&stopped_horn, &stopped_drum}) {
            const auto final_envelope = window_rms(*stopped, kSampleRate * 6U, 512);
            const auto [minimum, maximum] =
                std::minmax_element(final_envelope.begin(), final_envelope.end());
            assert(*maximum - *minimum < 0.002F);
        }
    }

    {
        bool threw = false;
        try {
            auto invalid = enabled(static_cast<RotaryVfxSpeed>(255));
            RotaryVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);

        threw = false;
        try {
            auto invalid = enabled(RotaryVfxSpeed::Slow);
            invalid.stereo_width_percent = 101;
            RotaryVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);

        threw = false;
        try {
            RotaryVfxProcessor processor({}, std::numeric_limits<std::uint32_t>::max(), kChannels);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);

        RotaryVfxProcessor processor(enabled(RotaryVfxSpeed::Slow), kSampleRate, kChannels);
        threw = false;
        try {
            processor.process_interleaved(nullptr, 1, kChannels);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);
    }

    {
        auto adjustment = enabled(RotaryVfxSpeed::Slow);
        RotaryVfxProcessor processor(adjustment, kSampleRate, kChannels);
        auto samples = fixture(4096);
        allocation_count.store(0, std::memory_order_relaxed);
        allocation_tracking.store(true, std::memory_order_relaxed);
        adjustment.speed = RotaryVfxSpeed::Fast;
        adjustment.motion_percent = 100;
        processor.update(adjustment);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        processor.reset();
        allocation_tracking.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    {
        RotaryVfxProcessor processor(enabled(RotaryVfxSpeed::Fast), kSampleRate, kChannels);
        auto samples = fixture(kSampleRate * 20U);
        const auto started = std::chrono::steady_clock::now();
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        const auto elapsed = std::chrono::steady_clock::now() - started;
        assert(elapsed < std::chrono::seconds(10));
    }

    return 0;
}
