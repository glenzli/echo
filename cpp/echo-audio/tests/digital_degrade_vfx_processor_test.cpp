#include "echo/audio/digital_degrade_vfx_processor.hpp"

#include <algorithm>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstdlib>
#include <iterator>
#include <new>
#include <stdexcept>
#include <vector>

namespace {

std::atomic<bool> track_allocations = false;
std::atomic<std::size_t> allocation_count = 0;

constexpr std::uint32_t kSampleRate = 48000;
constexpr float kPi = 3.14159265358979323846F;

echo::audio::DigitalDegradeVfxAdjustment
adjustment(echo::audio::DigitalDegradeVfxCharacter character) {
    echo::audio::DigitalDegradeVfxAdjustment value;
    value.enabled = true;
    value.character = character;
    value.mix_percent = 100;
    value.bitcrusher.bit_depth = 4;
    value.sample_rate_reduction.target_rate_hertz = 12000;
    return value;
}

std::vector<float> stereo_program(std::size_t frame_count) {
    std::vector<float> samples(frame_count * 2, 0.0F);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float time = static_cast<float>(frame) / static_cast<float>(kSampleRate);
        samples[frame * 2] = 0.77F * std::sin(2.0F * kPi * 431.0F * time);
        samples[frame * 2 + 1] = 0.63F * std::sin(2.0F * kPi * 719.0F * time + 0.4F);
    }
    return samples;
}

void expect_near(float actual, float expected, float tolerance = 1.0E-6F) {
    assert(std::abs(actual - expected) <= tolerance);
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
    using echo::audio::DigitalDegradeVfxCharacter;
    using echo::audio::DigitalDegradeVfxProcessor;

    static_assert(DigitalDegradeVfxProcessor::latency_frames() == 0);

    {
        const std::vector<float> input{-1.0F, -0.51F, -0.2F, 0.0F, 0.2F, 0.51F, 1.0F};
        auto output = input;
        auto settings = adjustment(DigitalDegradeVfxCharacter::Bitcrusher);
        DigitalDegradeVfxProcessor processor(settings, kSampleRate, 1);
        processor.process_interleaved(output.data(), output.size(), 1);
        assert(output != input);
        for (const float sample : output) {
            expect_near(sample * 7.0F, std::round(sample * 7.0F));
        }
        assert(output[3] == 0.0F);
        for (std::size_t index = 0; index < output.size(); ++index) {
            expect_near(output[index], -output[output.size() - 1 - index]);
        }
    }

    {
        std::vector<float> samples(24, 0.0F);
        for (std::size_t frame = 0; frame < 12; ++frame) {
            samples[frame * 2] = static_cast<float>(frame) / 12.0F;
            samples[frame * 2 + 1] = -static_cast<float>(frame) / 12.0F;
        }
        auto settings = adjustment(DigitalDegradeVfxCharacter::SampleRateReduction);
        DigitalDegradeVfxProcessor processor(settings, kSampleRate, 2);
        processor.process_interleaved(samples.data(), 12, 2);
        for (std::size_t group = 0; group < 3; ++group) {
            const std::size_t first = group * 4;
            for (std::size_t offset = 1; offset < 4; ++offset) {
                assert(samples[(first + offset) * 2] == samples[first * 2]);
                assert(samples[(first + offset) * 2 + 1] == samples[first * 2 + 1]);
            }
        }
        assert(samples[8] != samples[0]);
        assert(samples[9] != samples[1]);
    }

    {
        std::vector<float> samples(16, 0.0F);
        for (std::size_t frame = 0; frame < samples.size(); ++frame) {
            samples[frame] = 0.03F + static_cast<float>(frame) * 0.047F;
        }
        auto settings = adjustment(DigitalDegradeVfxCharacter::LoFi);
        DigitalDegradeVfxProcessor processor(settings, kSampleRate, 1);
        processor.process_interleaved(samples.data(), samples.size(), 1);
        for (std::size_t group = 0; group < 4; ++group) {
            const std::size_t first = group * 4;
            for (std::size_t offset = 1; offset < 4; ++offset) {
                assert(samples[first + offset] == samples[first]);
            }
            expect_near(samples[first] * 7.0F, std::round(samples[first] * 7.0F));
        }
    }

    for (const auto character : {
             DigitalDegradeVfxCharacter::Bitcrusher,
             DigitalDegradeVfxCharacter::SampleRateReduction,
             DigitalDegradeVfxCharacter::LoFi,
         }) {
        std::vector<float> silence(4096, 0.0F);
        DigitalDegradeVfxProcessor processor(adjustment(character), kSampleRate, 2);
        processor.process_interleaved(silence.data(), silence.size() / 2, 2);
        assert(std::all_of(silence.begin(), silence.end(), [](float sample) {
            return sample == 0.0F;
        }));
    }

    {
        auto input = stereo_program(4096);
        auto output = input;
        auto settings = adjustment(DigitalDegradeVfxCharacter::LoFi);
        settings.enabled = false;
        DigitalDegradeVfxProcessor processor(settings, kSampleRate, 2);
        assert(processor.is_bypassed());
        processor.process_interleaved(output.data(), output.size() / 2, 2);
        assert(output == input);

        settings.enabled = true;
        settings.mix_percent = 0;
        DigitalDegradeVfxProcessor zero_mix_processor(settings, kSampleRate, 2);
        output = input;
        zero_mix_processor.process_interleaved(output.data(), output.size() / 2, 2);
        assert(output == input);
    }

    {
        auto source = stereo_program(8192);
        auto contiguous = source;
        auto chunked = source;
        const auto settings = adjustment(DigitalDegradeVfxCharacter::LoFi);
        DigitalDegradeVfxProcessor whole(settings, kSampleRate, 2);
        DigitalDegradeVfxProcessor pieces(settings, kSampleRate, 2);
        whole.process_interleaved(contiguous.data(), contiguous.size() / 2, 2);
        const std::size_t chunks[]{1, 17, 251, 1024, 7, 509, 43};
        std::size_t frame = 0;
        std::size_t chunk = 0;
        while (frame < chunked.size() / 2) {
            const std::size_t count =
                std::min(chunks[chunk % std::size(chunks)], chunked.size() / 2 - frame);
            pieces.process_interleaved(chunked.data() + frame * 2, count, 2);
            frame += count;
            ++chunk;
        }
        assert(chunked == contiguous);
    }

    {
        std::vector<float> before(128, 0.37F);
        std::vector<float> after(128, 0.37F);
        auto settings = adjustment(DigitalDegradeVfxCharacter::Bitcrusher);
        settings.bitcrusher.bit_depth = 12;
        DigitalDegradeVfxProcessor processor(settings, kSampleRate, 1);
        processor.process_interleaved(before.data(), before.size(), 1);
        settings.character = DigitalDegradeVfxCharacter::LoFi;
        settings.bitcrusher.bit_depth = 2;
        settings.sample_rate_reduction.target_rate_hertz = 2000;
        processor.update(settings);
        processor.process_interleaved(after.data(), after.size(), 1);
        assert(std::abs(after.front() - before.back()) < 0.01F);
        for (std::size_t index = 1; index < after.size(); ++index) {
            assert(std::abs(after[index] - after[index - 1]) < 0.02F);
        }
    }

    {
        auto source = stereo_program(2048);
        auto after_reset = source;
        auto fresh_output = source;
        const auto settings = adjustment(DigitalDegradeVfxCharacter::LoFi);
        DigitalDegradeVfxProcessor reset_processor(settings, kSampleRate, 2);
        DigitalDegradeVfxProcessor fresh_processor(settings, kSampleRate, 2);
        auto warmup = stereo_program(333);
        reset_processor.process_interleaved(warmup.data(), warmup.size() / 2, 2);
        reset_processor.reset();
        reset_processor.process_interleaved(after_reset.data(), after_reset.size() / 2, 2);
        fresh_processor.process_interleaved(fresh_output.data(), fresh_output.size() / 2, 2);
        assert(after_reset == fresh_output);
    }

    {
        auto disabled = adjustment(DigitalDegradeVfxCharacter::SampleRateReduction);
        disabled.enabled = false;
        auto hidden = disabled;
        hidden.enabled = true;
        hidden.mix_percent = 0;
        DigitalDegradeVfxProcessor bypass_path(disabled, kSampleRate, 2);
        DigitalDegradeVfxProcessor zero_mix_path(hidden, kSampleRate, 2);
        auto prefix_a = stereo_program(317);
        auto prefix_b = prefix_a;
        bypass_path.process_interleaved(prefix_a.data(), prefix_a.size() / 2, 2);
        zero_mix_path.process_interleaved(prefix_b.data(), prefix_b.size() / 2, 2);
        assert(prefix_a == prefix_b);
        auto enabled = disabled;
        enabled.enabled = true;
        auto full_mix = hidden;
        full_mix.mix_percent = 100;
        bypass_path.update(enabled);
        zero_mix_path.update(full_mix);
        auto suffix_a = stereo_program(1000);
        auto suffix_b = suffix_a;
        bypass_path.process_interleaved(suffix_a.data(), suffix_a.size() / 2, 2);
        zero_mix_path.process_interleaved(suffix_b.data(), suffix_b.size() / 2, 2);
        assert(suffix_a == suffix_b);
    }

    {
        auto settings = adjustment(DigitalDegradeVfxCharacter::LoFi);
        DigitalDegradeVfxProcessor processor(settings, kSampleRate, 2);
        auto samples = stereo_program(4096);
        allocation_count.store(0, std::memory_order_relaxed);
        track_allocations.store(true, std::memory_order_relaxed);
        settings.character = DigitalDegradeVfxCharacter::Bitcrusher;
        settings.bitcrusher.bit_depth = 9;
        processor.update(settings);
        processor.process_interleaved(samples.data(), samples.size() / 2, 2);
        processor.reset();
        track_allocations.store(false, std::memory_order_relaxed);
        assert(allocation_count.load(std::memory_order_relaxed) == 0);
    }

    {
        bool rejected = false;
        auto invalid = adjustment(DigitalDegradeVfxCharacter::Bitcrusher);
        invalid.bitcrusher.bit_depth = 1;
        try {
            DigitalDegradeVfxProcessor processor(invalid, kSampleRate, 2);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    return 0;
}
