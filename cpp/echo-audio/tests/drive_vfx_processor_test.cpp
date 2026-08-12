#include "echo/audio/drive_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <iterator>
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

echo::audio::DriveVfxAdjustment enabled(echo::audio::DriveVfxCharacter character) {
    return {
        .character = character,
        .enabled = true,
        .mix_percent = 100,
        .drive_centibels = 1800,
        .tone_hertz = 12000,
        .output_gain_centibels = -500,
    };
}

std::vector<float> fixture(std::size_t frame_count, std::size_t channel_count = kChannels) {
    std::vector<float> samples(frame_count * channel_count, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * channel_count] = 0.72F * std::sin(2.0F * kPi * 431.0F * time)
                                         + 0.18F * std::sin(2.0F * kPi * 3113.0F * time);
        if (channel_count == 2) {
            samples[frame * channel_count + 1] = 0.61F * std::sin(2.0F * kPi * 719.0F * time + 0.4F)
                                                 - 0.13F * std::sin(2.0F * kPi * 4201.0F * time);
        }
    }
    return samples;
}

std::vector<float> process(
    echo::audio::DriveVfxAdjustment adjustment,
    const std::vector<float>& input,
    std::size_t channel_count,
    std::size_t block_frames
) {
    echo::audio::DriveVfxProcessor processor(adjustment, kSampleRate, channel_count);
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

float spectral_amplitude(
    const std::vector<float>& samples,
    std::size_t first_frame,
    std::size_t frame_count,
    float frequency_hertz
) {
    double real = 0.0;
    double imaginary = 0.0;
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const double phase = 2.0 * static_cast<double>(kPi) * static_cast<double>(frequency_hertz)
                             * static_cast<double>(frame) / static_cast<double>(kSampleRate);
        const double sample = static_cast<double>(samples[first_frame + frame]);
        real += sample * std::cos(phase);
        imaginary -= sample * std::sin(phase);
    }
    return static_cast<float>(
        2.0 * std::sqrt(real * real + imaginary * imaginary) / static_cast<double>(frame_count)
    );
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
    using echo::audio::DriveVfxCharacter;
    using echo::audio::DriveVfxProcessor;

    static_assert(DriveVfxProcessor::latency_frames() == 32);
    const auto input = fixture(8192);

    {
        DriveVfxProcessor processor({}, kSampleRate, kChannels);
        auto output = input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(processor.is_bypassed());
        for (std::size_t frame = 0; frame < output.size() / kChannels; ++frame) {
            for (std::size_t channel = 0; channel < kChannels; ++channel) {
                const float expected =
                    frame < DriveVfxProcessor::latency_frames()
                        ? 0.0F
                        : input
                              [(frame - DriveVfxProcessor::latency_frames()) * kChannels + channel];
                assert(output[frame * kChannels + channel] == expected);
            }
        }
    }

    {
        auto adjustment = enabled(DriveVfxCharacter::Overdrive);
        adjustment.mix_percent = 0;
        const auto output = process(adjustment, input, kChannels, 257);
        for (std::size_t frame = DriveVfxProcessor::latency_frames();
             frame < output.size() / kChannels;
             ++frame) {
            assert(
                output[frame * kChannels]
                == input[(frame - DriveVfxProcessor::latency_frames()) * kChannels]
            );
        }
    }

    constexpr std::array characters{
        DriveVfxCharacter::SoftClip,
        DriveVfxCharacter::Overdrive,
        DriveVfxCharacter::Fuzz,
    };
    std::array<std::vector<float>, characters.size()> outputs;
    for (std::size_t index = 0; index < characters.size(); ++index) {
        const auto adjustment = enabled(characters[index]);
        outputs[index] = process(adjustment, input, kChannels, 257);
        assert(std::all_of(outputs[index].begin(), outputs[index].end(), [](float sample) {
            return std::isfinite(sample) && std::abs(sample) < 4.0F;
        }));
        assert_near(outputs[index], process(adjustment, input, kChannels, 1));
        assert_near(outputs[index], process(adjustment, input, kChannels, 13));

        const auto mono = fixture(4096, 1);
        const auto mono_output = process(adjustment, mono, 1, 137);
        assert(mono_output != mono);

        std::vector<float> silence(4096 * kChannels, 0.0F);
        const auto silent_output = process(adjustment, silence, kChannels, 31);
        assert(std::all_of(silent_output.begin(), silent_output.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }
    for (std::size_t left = 0; left < outputs.size(); ++left) {
        for (std::size_t right = left + 1; right < outputs.size(); ++right) {
            bool differs = false;
            for (std::size_t index = DriveVfxProcessor::latency_frames() * kChannels;
                 index < outputs[left].size();
                 ++index) {
                differs =
                    differs || std::abs(outputs[left][index] - outputs[right][index]) > 1.0E-4F;
            }
            assert(differs);
        }
    }

    {
        auto left_only = fixture(4096);
        for (std::size_t frame = 0; frame < left_only.size() / kChannels; ++frame) {
            left_only[frame * kChannels + 1] = 0.0F;
        }
        const auto output = process(enabled(DriveVfxCharacter::Fuzz), left_only, kChannels, 257);
        for (std::size_t frame = 0; frame < output.size() / kChannels; ++frame) {
            assert(output[frame * kChannels + 1] == 0.0F);
        }
    }

    {
        std::vector<float> hostile(128 * kChannels, 0.0F);
        hostile[0] = std::numeric_limits<float>::quiet_NaN();
        hostile[1] = std::numeric_limits<float>::infinity();
        hostile[2] = -std::numeric_limits<float>::infinity();
        hostile[3] = std::numeric_limits<float>::max();
        hostile[4] = -std::numeric_limits<float>::max();
        hostile[5] = 0.5F;
        DriveVfxProcessor processor(enabled(DriveVfxCharacter::Overdrive), kSampleRate, kChannels);
        processor.process_interleaved(hostile.data(), hostile.size() / kChannels, kChannels);
        assert(std::all_of(hostile.begin(), hostile.end(), [](float sample) {
            return std::isfinite(sample);
        }));
    }

    {
        auto adjustment = enabled(DriveVfxCharacter::SoftClip);
        DriveVfxProcessor used(adjustment, kSampleRate, kChannels);
        DriveVfxProcessor fresh(adjustment, kSampleRate, kChannels);
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
        auto adjustment = enabled(DriveVfxCharacter::SoftClip);
        DriveVfxProcessor transitioning(adjustment, kSampleRate, kChannels);
        assert(transitioning.adjustment().character == DriveVfxCharacter::SoftClip);
        DriveVfxProcessor reference(adjustment, kSampleRate, kChannels);
        auto warmup_a = fixture(4096);
        auto warmup_b = warmup_a;
        transitioning.process_interleaved(warmup_a.data(), warmup_a.size() / kChannels, kChannels);
        reference.process_interleaved(warmup_b.data(), warmup_b.size() / kChannels, kChannels);
        adjustment.character = DriveVfxCharacter::Fuzz;
        adjustment.drive_centibels = 3600;
        adjustment.tone_hertz = 1000;
        adjustment.output_gain_centibels = -1200;
        transitioning.update(adjustment);
        assert(transitioning.adjustment().character == DriveVfxCharacter::Fuzz);
        auto next_a = fixture(1);
        auto next_b = next_a;
        transitioning.process_interleaved(next_a.data(), 1, kChannels);
        reference.process_interleaved(next_b.data(), 1, kChannels);
        assert(std::abs(next_a[0] - next_b[0]) < 0.01F);
        assert(std::abs(next_a[1] - next_b[1]) < 0.01F);
    }

    {
        constexpr std::size_t kFrames = 32768;
        std::vector<float> input_tone(kFrames, 0.0F);
        std::vector<float> naive(kFrames, 0.0F);
        for (std::size_t frame = 0; frame < kFrames; ++frame) {
            const float sample = 0.9F
                                 * std::sin(
                                     2.0F * kPi * 13000.0F * static_cast<float>(frame)
                                     / static_cast<float>(kSampleRate)
                                 );
            input_tone[frame] = sample;
            naive[frame] = std::clamp(sample * 94.64F, -1.0F, 1.0F);
        }
        auto adjustment = enabled(DriveVfxCharacter::Fuzz);
        adjustment.drive_centibels = 3600;
        adjustment.tone_hertz = 16000;
        adjustment.output_gain_centibels = 0;
        DriveVfxProcessor processor(adjustment, kSampleRate, 1);
        processor.process_interleaved(input_tone.data(), input_tone.size(), 1);
        constexpr std::size_t kSkip = 4096;
        constexpr std::size_t kAnalysisFrames = 24000;
        const float antialiased_foldback =
            spectral_amplitude(input_tone, kSkip, kAnalysisFrames, 9000.0F);
        const float naive_foldback = spectral_amplitude(naive, kSkip, kAnalysisFrames, 9000.0F);
        assert(antialiased_foldback < naive_foldback * 0.15F);
    }

    {
        auto adjustment = enabled(DriveVfxCharacter::Overdrive);
        DriveVfxProcessor processor(adjustment, kSampleRate, kChannels);
        std::array<float, 2048> samples{};
        for (std::size_t index = 0; index < samples.size(); ++index) {
            samples[index] = 0.4F * std::sin(static_cast<float>(index) * 0.023F);
        }
        allocation_count.store(0, std::memory_order_relaxed);
        allocation_tracking.store(true, std::memory_order_relaxed);
        adjustment.character = DriveVfxCharacter::Fuzz;
        processor.update(adjustment);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        processor.reset();
        allocation_tracking.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    {
        bool rejected = false;
        try {
            auto invalid = enabled(DriveVfxCharacter::SoftClip);
            invalid.character = static_cast<DriveVfxCharacter>(255);
            DriveVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            auto invalid = enabled(DriveVfxCharacter::SoftClip);
            invalid.mix_percent = 101;
            DriveVfxProcessor invalid_processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            auto invalid = enabled(DriveVfxCharacter::SoftClip);
            invalid.tone_hertz = 499;
            DriveVfxProcessor invalid_processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            auto invalid = enabled(DriveVfxCharacter::SoftClip);
            invalid.output_gain_centibels = -2401;
            DriveVfxProcessor invalid_processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            auto invalid = enabled(DriveVfxCharacter::SoftClip);
            invalid.drive_centibels = 3601;
            DriveVfxProcessor processor(invalid, kSampleRate, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        rejected = false;
        try {
            DriveVfxProcessor invalid_processor({}, kSampleRate, 3);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);

        DriveVfxProcessor processor({}, kSampleRate, kChannels);
        rejected = false;
        try {
            processor.process_interleaved(nullptr, 1, kChannels);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    return 0;
}
