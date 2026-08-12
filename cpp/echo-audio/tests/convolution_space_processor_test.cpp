#include "echo/audio/convolution_space_processor.hpp"

#include <algorithm>
#include <array>
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
std::atomic<bool> allocation_tracking = false;
std::atomic<std::size_t> allocation_count = 0;

echo::audio::ConvolutionSpaceAdjustment enabled() {
    return {.enabled = true, .mix_percent = 100, .wet_gain_centibels = 0};
}

std::vector<float> fixture(std::size_t frame_count, std::size_t channel_count = kChannels) {
    std::vector<float> samples(frame_count * channel_count, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * channel_count] = 0.4F * std::sin(2.0F * kPi * 337.0F * time)
                                         + 0.1F * std::sin(2.0F * kPi * 2011.0F * time);
        if (channel_count == 2) {
            samples[frame * channel_count + 1] = 0.33F * std::sin(2.0F * kPi * 521.0F * time + 0.2F)
                                                 - 0.08F * std::sin(2.0F * kPi * 3109.0F * time);
        }
    }
    return samples;
}

std::vector<float> process(
    echo::audio::ConvolutionSpaceAdjustment adjustment,
    const std::vector<float>& input,
    std::size_t channel_count,
    std::span<const float> impulse_left,
    std::span<const float> impulse_right,
    std::size_t block_frames
) {
    echo::audio::ConvolutionSpaceProcessor
        processor(adjustment, kSampleRate, channel_count, impulse_left, impulse_right);
    auto output = input;
    const std::size_t frames = output.size() / channel_count;
    std::size_t offset = 0;
    while (offset < frames) {
        const std::size_t count = std::min(block_frames, frames - offset);
        processor.process_interleaved(output.data() + offset * channel_count, count, channel_count);
        offset += count;
    }
    return output;
}

std::vector<float>
direct_convolution_prefix(const std::vector<float>& input, const std::vector<float>& impulse) {
    std::vector<float> output(input.size(), 0.0F);
    for (std::size_t frame = 0; frame < input.size(); ++frame) {
        double sum = 0.0;
        const std::size_t last_tap = std::min(frame, impulse.size() - 1);
        for (std::size_t tap = 0; tap <= last_tap; ++tap) {
            sum += static_cast<double>(input[frame - tap]) * static_cast<double>(impulse[tap]);
        }
        output[frame] = static_cast<float>(sum);
    }
    return output;
}

void assert_near(
    const std::vector<float>& actual,
    const std::vector<float>& expected,
    float tolerance = 2.0E-5F
) {
    assert(actual.size() == expected.size());
    for (std::size_t index = 0; index < actual.size(); ++index) {
        assert(std::abs(actual[index] - expected[index]) <= tolerance);
    }
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
    using echo::audio::ConvolutionSpaceProcessor;
    using echo::audio::PreparedImpulseLayout;

    static_assert(ConvolutionSpaceProcessor::latency_frames() == 0);
    const std::vector<float> delta{1.0F};
    const auto stereo_input = fixture(8192);

    {
        ConvolutionSpaceProcessor processor({}, kSampleRate, kChannels, delta);
        auto output = stereo_input;
        processor.process_interleaved(output.data(), output.size() / kChannels, kChannels);
        assert(processor.is_bypassed());
        assert(processor.impulse_layout() == PreparedImpulseLayout::Mono);
        assert(processor.impulse_frames() == 1);
        assert(processor.tail_frames() == 0);
        assert(output == stereo_input);
    }

    {
        const auto output = process(enabled(), stereo_input, kChannels, delta, {}, 257);
        assert_near(output, stereo_input, 3.0E-6F);

        std::vector<float> dc(1024, 1.0F);
        assert_near(process(enabled(), dc, 1, delta, {}, 256), dc, 3.0E-6F);
        std::vector<float> nyquist(1024, 1.0F);
        for (std::size_t frame = 1; frame < nyquist.size(); frame += 2) {
            nyquist[frame] = -1.0F;
        }
        assert_near(process(enabled(), nyquist, 1, delta, {}, 256), nyquist, 3.0E-6F);

        const std::vector<float> quiet_tail_ir{1.0F, 5.0E-7F};
        std::vector<float> unit_impulse(512, 0.0F);
        unit_impulse[0] = 1.0F;
        const auto quiet_tail = process(enabled(), unit_impulse, 1, quiet_tail_ir, {}, 256);
        assert(std::abs(quiet_tail[1] - 5.0E-7F) < 2.0E-7F);
    }

    const std::vector<float> impulse{0.75F, -0.25F, 0.125F, 0.0625F};
    const auto mono_input = fixture(4096, 1);
    const auto oracle = direct_convolution_prefix(mono_input, impulse);
    const auto reference = process(enabled(), mono_input, 1, impulse, {}, 4096);
    assert_near(reference, oracle);
    for (const std::size_t block_frames : {1U, 127U, 256U, 4096U}) {
        assert_near(reference, process(enabled(), mono_input, 1, impulse, {}, block_frames));
    }

    {
        const std::vector<float> left_ir{1.0F};
        const std::vector<float> right_ir{0.0F};
        const auto output = process(enabled(), stereo_input, kChannels, left_ir, right_ir, 257);
        for (std::size_t frame = 0; frame < output.size() / kChannels; ++frame) {
            assert(std::abs(output[frame * kChannels] - stereo_input[frame * kChannels]) < 3.0E-6F);
            assert(std::abs(output[frame * kChannels + 1]) < 3.0E-6F);
        }
        ConvolutionSpaceProcessor processor(enabled(), kSampleRate, kChannels, left_ir, right_ir);
        assert(processor.impulse_layout() == PreparedImpulseLayout::StereoParallel);
    }

    {
        std::vector<float> silence(8192 * kChannels, 0.0F);
        const auto output = process(enabled(), silence, kChannels, impulse, {}, 31);
        assert(std::all_of(output.begin(), output.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }

    {
        ConvolutionSpaceProcessor used(enabled(), kSampleRate, kChannels, impulse);
        ConvolutionSpaceProcessor fresh(enabled(), kSampleRate, kChannels, impulse);
        auto discarded = fixture(777);
        used.process_interleaved(discarded.data(), discarded.size() / kChannels, kChannels);
        used.reset();
        auto reset_output = stereo_input;
        auto fresh_output = stereo_input;
        used.process_interleaved(reset_output.data(), reset_output.size() / kChannels, kChannels);
        fresh.process_interleaved(fresh_output.data(), fresh_output.size() / kChannels, kChannels);
        assert_near(reset_output, fresh_output);
    }

    {
        std::vector<float> hostile(128 * kChannels, 0.0F);
        hostile[0] = std::numeric_limits<float>::quiet_NaN();
        hostile[1] = std::numeric_limits<float>::infinity();
        hostile[2] = -std::numeric_limits<float>::infinity();
        ConvolutionSpaceProcessor processor(enabled(), kSampleRate, kChannels, impulse);
        processor.process_interleaved(hostile.data(), hostile.size() / kChannels, kChannels);
        assert(std::all_of(hostile.begin(), hostile.end(), [](float sample) {
            return std::isfinite(sample);
        }));
    }

    {
        bool threw = false;
        try {
            ConvolutionSpaceProcessor processor(enabled(), 44100, 1, impulse);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);

        threw = false;
        try {
            const std::vector<float> silent_ir(16, 0.0F);
            ConvolutionSpaceProcessor processor(enabled(), kSampleRate, 1, silent_ir);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);

        threw = false;
        try {
            const std::vector<float> invalid_ir{1.0F, std::numeric_limits<float>::quiet_NaN()};
            ConvolutionSpaceProcessor processor(enabled(), kSampleRate, 1, invalid_ir);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        assert(threw);
    }

    {
        auto adjustment = enabled();
        ConvolutionSpaceProcessor processor(adjustment, kSampleRate, kChannels, impulse);
        auto samples = fixture(4096);
        allocation_count.store(0, std::memory_order_relaxed);
        allocation_tracking.store(true, std::memory_order_relaxed);
        adjustment.mix_percent = 60;
        adjustment.wet_gain_centibels = -600;
        processor.update(adjustment);
        processor.process_interleaved(samples.data(), samples.size() / kChannels, kChannels);
        processor.reset();
        allocation_tracking.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    {
        auto adjustment = enabled();
        ConvolutionSpaceProcessor processor(adjustment, kSampleRate, 1, impulse);
        auto active = fixture(1024, 1);
        processor.process_interleaved(active.data(), active.size(), 1);
        adjustment.enabled = false;
        processor.update(adjustment);
        assert(!processor.is_bypassed());
        auto fade = fixture(960, 1);
        processor.process_interleaved(fade.data(), fade.size(), 1);
        assert(!processor.is_bypassed());
        auto stable_dry = fixture(2048, 1);
        const auto expected_dry = stable_dry;
        processor.process_interleaved(stable_dry.data(), stable_dry.size(), 1);
        assert(processor.is_bypassed());
        assert(stable_dry == expected_dry);

        adjustment.enabled = true;
        processor.update(adjustment);
        auto resumed = fixture(2048, 1);
        processor.process_interleaved(resumed.data(), resumed.size(), 1);
        assert(std::all_of(resumed.begin(), resumed.end(), [](float sample) {
            return std::isfinite(sample);
        }));
    }

    {
        std::vector<float> delayed_ir(2049, 0.0F);
        delayed_ir[0] = 0.5F;
        delayed_ir[2048] = 0.5F;
        auto adjustment = enabled();
        ConvolutionSpaceProcessor processor(adjustment, kSampleRate, 1, delayed_ir);
        std::vector<float> impulse_input(256, 0.0F);
        impulse_input[0] = 1.0F;
        processor.process_interleaved(impulse_input.data(), impulse_input.size(), 1);
        adjustment.enabled = false;
        processor.update(adjustment);
        std::vector<float> fade(960, 0.0F);
        processor.process_interleaved(fade.data(), fade.size(), 1);
        std::vector<float> clear_trigger(1, 0.0F);
        processor.process_interleaved(clear_trigger.data(), clear_trigger.size(), 1);
        assert(processor.is_bypassed());
        adjustment.enabled = true;
        processor.update(adjustment);
        std::vector<float> zero_after_resume(4096, 0.0F);
        processor.process_interleaved(zero_after_resume.data(), zero_after_resume.size(), 1);
        assert(std::all_of(zero_after_resume.begin(), zero_after_resume.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }

    return 0;
}
