#include "echo/audio/spring_space_reverb.hpp"

#include <algorithm>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <limits>
#include <new>
#include <vector>

namespace {

std::atomic<std::size_t> allocation_count{0};

echo::audio::ReverbAdjustment spring_adjustment() {
    return {
        .character = echo::audio::ReverbCharacter::Spring,
        .enabled = true,
        .mix_percent = 100,
        .pre_delay_millis = 0,
        .decay_millis = 1'800,
        .size_percent = 55,
        .damping_percent = 45,
        .low_cut_hertz = 120,
        .high_cut_hertz = 10'000,
    };
}

void assert_finite(const std::vector<float>& samples) {
    for (const float sample : samples) {
        assert(std::isfinite(sample));
    }
}

void test_silence_reset_and_latency() {
    echo::audio::SpringSpaceReverb::validate(spring_adjustment(), 48'000, 2);
    echo::audio::SpringSpaceReverb reverb(spring_adjustment(), 48'000, 2);
    assert(reverb.latency_frames() == 0);
    assert(!reverb.is_bypassed());

    std::vector<float> silence(8'192, 0.0F);
    reverb.process_interleaved(silence.data(), silence.size() / 2, 2);
    assert(std::all_of(silence.begin(), silence.end(), [](float sample) {
        return sample == 0.0F;
    }));

    silence[0] = 1.0F;
    silence[1] = 1.0F;
    reverb.process_interleaved(silence.data(), silence.size() / 2, 2);
    reverb.reset();
    std::fill(silence.begin(), silence.end(), 0.0F);
    reverb.process_interleaved(silence.data(), silence.size() / 2, 2);
    assert(std::all_of(silence.begin(), silence.end(), [](float sample) {
        return sample == 0.0F;
    }));
}

void test_frame_and_block_entry_points_match() {
    const auto adjustment = spring_adjustment();
    echo::audio::SpringSpaceReverb by_frame(adjustment, 48'000, 2);
    echo::audio::SpringSpaceReverb by_block(adjustment, 48'000, 2);
    std::vector<float> input(4'096, 0.0F);
    input[0] = 0.8F;
    input[1] = -0.4F;
    std::vector<float> expected = input;
    by_block.process_interleaved(expected.data(), expected.size() / 2, 2);
    for (std::size_t frame = 0; frame < input.size() / 2; ++frame) {
        const auto output = by_frame.process_frame(input[2 * frame], input[2 * frame + 1]);
        assert(std::abs(output[0] - expected[2 * frame]) < 1.0e-7F);
        assert(std::abs(output[1] - expected[2 * frame + 1]) < 1.0e-7F);
    }
}

void test_mix_and_enabled_updates_are_smoothed() {
    auto adjustment = spring_adjustment();
    adjustment.mix_percent = 0;
    echo::audio::SpringSpaceReverb reverb(adjustment, 48'000, 1);
    std::vector<float> signal(1'100, 0.25F);
    adjustment.mix_percent = 100;
    reverb.update(adjustment);
    reverb.process_interleaved(signal.data(), signal.size(), 1);
    assert(signal.front() > 0.24F);
    for (std::size_t index = 1; index < 960; ++index) {
        assert(std::abs(signal[index] - signal[index - 1]) < 0.001F);
    }
    assert(std::abs(signal[959]) < 0.001F);

    reverb.reset();
    adjustment.enabled = false;
    reverb.update(adjustment);
    std::fill(signal.begin(), signal.end(), 0.25F);
    reverb.process_interleaved(signal.data(), signal.size(), 1);
    assert(signal.front() < 0.01F);
    assert(std::abs(signal[959] - 0.25F) < 0.001F);
}

void test_impulse_has_dispersive_stereo_tail() {
    echo::audio::SpringSpaceReverb reverb(spring_adjustment(), 48'000, 2);
    std::vector<float> response(48'000 * 2, 0.0F);
    response[0] = 1.0F;
    response[1] = 1.0F;
    reverb.process_interleaved(response.data(), 48'000, 2);
    assert_finite(response);

    std::size_t first_wet_frame = 48'000;
    bool stereo_difference = false;
    double tail_energy = 0.0;
    for (std::size_t frame = 0; frame < 48'000; ++frame) {
        const float left = response[2 * frame];
        const float right = response[2 * frame + 1];
        if (first_wet_frame == 48'000 && (std::abs(left) > 1.0e-8F || std::abs(right) > 1.0e-8F)) {
            first_wet_frame = frame;
        }
        stereo_difference = stereo_difference || std::abs(left - right) > 1.0e-7F;
        if (frame > 2'000) {
            tail_energy += static_cast<double>(left) * left + static_cast<double>(right) * right;
        }
    }
    assert(first_wet_frame > 800);
    assert(first_wet_frame < 2'200);
    assert(stereo_difference);
    assert(tail_energy > 1.0e-8);
}

void test_block_partition_invariance() {
    const auto adjustment = spring_adjustment();
    echo::audio::SpringSpaceReverb contiguous(adjustment, 48'000, 2);
    echo::audio::SpringSpaceReverb partitioned(adjustment, 48'000, 2);
    std::vector<float> expected(16'384, 0.0F);
    for (std::size_t index = 0; index < expected.size(); ++index) {
        expected[index] = 0.2F * std::sin(static_cast<float>(index) * 0.017F)
                          + 0.1F * std::cos(static_cast<float>(index) * 0.031F);
    }
    std::vector<float> actual = expected;
    contiguous.process_interleaved(expected.data(), expected.size() / 2, 2);
    constexpr std::size_t partitions[]{1, 64, 257, 4'096};
    std::size_t frame = 0;
    std::size_t partition = 0;
    while (frame < actual.size() / 2) {
        const std::size_t count = std::min(partitions[partition % 4], actual.size() / 2 - frame);
        partitioned.process_interleaved(actual.data() + 2 * frame, count, 2);
        frame += count;
        ++partition;
    }
    for (std::size_t index = 0; index < actual.size(); ++index) {
        assert(std::abs(actual[index] - expected[index]) < 1.0e-7F);
    }
}

void test_mono_updates_and_allocation_free_execution() {
    auto adjustment = spring_adjustment();
    echo::audio::SpringSpaceReverb reverb(adjustment, 48'000, 1);
    std::vector<float> samples(12'000, 0.0F);
    samples[0] = 1.0F;
    samples[3'000] = 1.0F;

    const std::size_t before = allocation_count.load();
    adjustment.mix_percent = 35;
    reverb.update(adjustment);
    adjustment.size_percent = 80;
    adjustment.decay_millis = 4'000;
    reverb.update(adjustment);
    reverb.process_interleaved(samples.data(), 3'000, 1);
    const auto zero_frame = reverb.process_frame(0.0F, 0.0F);
    assert(std::isfinite(zero_frame[0]));
    assert(zero_frame[0] == zero_frame[1]);
    adjustment.size_percent = 25;
    reverb.update(adjustment);
    adjustment.size_percent = 100;
    adjustment.damping_percent = 15;
    reverb.update(adjustment);
    adjustment.size_percent = 70;
    adjustment.decay_millis = 2'600;
    reverb.update(adjustment);
    reverb.reset();
    reverb.process_interleaved(samples.data() + 3'000, 9'000, 1);
    assert(allocation_count.load() == before);
    assert_finite(samples);

    echo::audio::SpringSpaceReverb expected_reverb(adjustment, 48'000, 1);
    std::vector<float> expected(9'000, 0.0F);
    expected[0] = 1.0F;
    expected_reverb.process_interleaved(expected.data(), expected.size(), 1);
    for (std::size_t index = 0; index < expected.size(); ++index) {
        assert(std::abs(samples[index + 3'000] - expected[index]) < 1.0e-7F);
    }

    adjustment.enabled = false;
    reverb.update(adjustment);
    assert(reverb.is_bypassed());
}

void test_worst_case_state_remains_finite_and_bounded() {
    auto adjustment = spring_adjustment();
    adjustment.decay_millis = 12'000;
    adjustment.size_percent = 100;
    adjustment.damping_percent = 0;
    adjustment.low_cut_hertz = 20;
    adjustment.high_cut_hertz = 20'000;
    echo::audio::SpringSpaceReverb reverb(adjustment, 48'000, 2);

    std::vector<float> block(257 * 2, 0.0F);
    block[0] = 1.0F;
    block[1] = -1.0F;
    float maximum = 0.0F;
    for (std::size_t processed = 0; processed < 48'000U * 8U; processed += 257U) {
        reverb.process_interleaved(block.data(), 257, 2);
        for (const float sample : block) {
            assert(std::isfinite(sample));
            maximum = std::max(maximum, std::abs(sample));
        }
        std::fill(block.begin(), block.end(), 0.0F);
    }
    assert(maximum > 0.0F);
    assert(maximum < 8.0F);
}

} // namespace

void* operator new(std::size_t size) {
    allocation_count.fetch_add(1);
    if (void* memory = std::malloc(size)) {
        return memory;
    }
    throw std::bad_alloc();
}

void* operator new[](std::size_t size) {
    allocation_count.fetch_add(1);
    if (void* memory = std::malloc(size)) {
        return memory;
    }
    throw std::bad_alloc();
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
    test_silence_reset_and_latency();
    test_frame_and_block_entry_points_match();
    test_mix_and_enabled_updates_are_smoothed();
    test_impulse_has_dispersive_stereo_tail();
    test_block_partition_invariance();
    test_mono_updates_and_allocation_free_execution();
    test_worst_case_state_remains_finite_and_bounded();
    return 0;
}
