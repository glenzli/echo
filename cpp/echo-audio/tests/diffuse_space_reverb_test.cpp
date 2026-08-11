#include "echo/audio/diffuse_space_reverb.hpp"

#include <array>
#include <atomic>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <new>
#include <stdexcept>
#include <vector>

namespace {

std::atomic<bool> g_track_allocations = false;
std::atomic<std::size_t> g_allocation_count = 0;

constexpr std::uint32_t kSampleRate = 48000;
constexpr std::size_t kFrameCount = kSampleRate * 3;

struct StereoRender {
    std::vector<float> left;
    std::vector<float> right;
};

echo::audio::ReverbAdjustment adjustment(echo::audio::ReverbCharacter character) {
    echo::audio::ReverbAdjustment value;
    value.character = character;
    value.enabled = true;
    value.mix_percent = 100;
    value.pre_delay_millis = 12;
    value.decay_millis = 2'800;
    value.size_percent = 70;
    value.damping_percent = 35;
    return value;
}

StereoRender render_impulse(echo::audio::ReverbCharacter character) {
    echo::audio::DiffuseSpaceReverb reverb(adjustment(character), kSampleRate);
    StereoRender render{
        .left = std::vector<float>(kFrameCount, 0.0F),
        .right = std::vector<float>(kFrameCount, 0.0F),
    };
    for (std::size_t frame = 0; frame < kFrameCount; ++frame) {
        const auto output = reverb.process_frame(frame == 0 ? 1.0F : 0.0F, 0.0F);
        render.left[frame] = output[0];
        render.right[frame] = output[1];
    }
    return render;
}

float energy(const std::vector<float>& samples, std::size_t first_frame) {
    float result = 0.0F;
    for (std::size_t frame = first_frame; frame < samples.size(); ++frame) {
        result += samples[frame] * samples[frame];
    }
    return result;
}

} // namespace

void* operator new(std::size_t size) {
    if (g_track_allocations.load(std::memory_order_relaxed)) {
        g_allocation_count.fetch_add(1, std::memory_order_relaxed);
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
    const StereoRender hall = render_impulse(echo::audio::ReverbCharacter::Hall);
    const StereoRender plate = render_impulse(echo::audio::ReverbCharacter::Plate);
    assert(energy(hall.left, 1) > 0.0001F);
    assert(energy(hall.right, 1) > 0.0001F);
    assert(energy(plate.left, 1) > 0.0001F);
    assert(energy(plate.right, 1) > 0.0001F);
    for (std::size_t frame = 0; frame < 12 * kSampleRate / 1000; ++frame) {
        assert(hall.left[frame] == 0.0F);
        assert(hall.right[frame] == 0.0F);
        assert(plate.left[frame] == 0.0F);
        assert(plate.right[frame] == 0.0F);
    }

    float hall_stereo_difference = 0.0F;
    float character_difference = 0.0F;
    for (std::size_t frame = 0; frame < kFrameCount; ++frame) {
        assert(std::isfinite(hall.left[frame]));
        assert(std::isfinite(hall.right[frame]));
        assert(std::isfinite(plate.left[frame]));
        assert(std::isfinite(plate.right[frame]));
        assert(std::abs(hall.left[frame]) < 4.0F);
        assert(std::abs(hall.right[frame]) < 4.0F);
        assert(std::abs(plate.left[frame]) < 4.0F);
        assert(std::abs(plate.right[frame]) < 4.0F);
        hall_stereo_difference += std::abs(hall.left[frame] - hall.right[frame]);
        character_difference += std::abs(hall.left[frame] - plate.left[frame]);
    }
    assert(hall_stereo_difference > 0.01F);
    assert(character_difference > 0.01F);

    auto hall_adjustment = adjustment(echo::audio::ReverbCharacter::Hall);
    echo::audio::DiffuseSpaceReverb prepared(hall_adjustment, kSampleRate);
    g_allocation_count.store(0, std::memory_order_relaxed);
    g_track_allocations.store(true, std::memory_order_relaxed);
    std::array<float, 2> last{};
    for (std::size_t frame = 0; frame < kSampleRate; ++frame) {
        last = prepared.process_frame(frame == 0 ? 1.0F : 0.0F, 0.0F);
    }
    g_track_allocations.store(false, std::memory_order_relaxed);
    assert(g_allocation_count.load(std::memory_order_relaxed) == 0);
    assert(std::isfinite(last[0]));
    assert(std::isfinite(last[1]));

    auto worst_case = adjustment(echo::audio::ReverbCharacter::Hall);
    worst_case.pre_delay_millis = 200;
    worst_case.decay_millis = 12'000;
    worst_case.size_percent = 100;
    worst_case.damping_percent = 0;
    echo::audio::DiffuseSpaceReverb stable(worst_case, kSampleRate);
    for (std::size_t frame = 0; frame < kSampleRate * 10; ++frame) {
        const auto output = stable.process_frame(frame == 0 ? 1.0F : 0.0F, 0.0F);
        assert(std::isfinite(output[0]));
        assert(std::isfinite(output[1]));
        assert(std::abs(output[0]) < 4.0F);
        assert(std::abs(output[1]) < 4.0F);
    }

    hall_adjustment.enabled = false;
    echo::audio::DiffuseSpaceReverb bypassed(hall_adjustment, kSampleRate);
    const auto dry = bypassed.process_frame(0.5F, -0.25F);
    assert(dry[0] == 0.5F);
    assert(dry[1] == -0.25F);

    prepared.reset();
    for (std::size_t frame = 0; frame < kSampleRate; ++frame) {
        const auto silent = prepared.process_frame(0.0F, 0.0F);
        assert(silent[0] == 0.0F);
        assert(silent[1] == 0.0F);
    }

    bool rejected = false;
    try {
        echo::audio::DiffuseSpaceReverb invalid({}, kSampleRate);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
}
