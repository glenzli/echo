#include "echo/audio/algorithmic_reverb.hpp"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <stdexcept>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;

float tail_energy(const std::vector<float>& samples, std::size_t first_frame) {
    float energy = 0.0F;
    for (std::size_t index = first_frame * 2; index < samples.size(); ++index) {
        energy += samples[index] * samples[index];
    }
    return energy;
}

void assert_near(float actual, float expected, float tolerance) {
    assert(std::abs(actual - expected) < tolerance);
}

} // namespace

int main() {
    std::vector<float> dry(4096 * 2, 0.0F);
    dry[0] = 0.5F;
    dry[1] = -0.25F;
    echo::audio::AlgorithmicReverb bypassed({}, kSampleRate, 2);
    bypassed.process_interleaved(dry.data(), 4096, 2);
    assert(dry[0] == 0.5F);
    assert(dry[1] == -0.25F);
    assert(tail_energy(dry, 1) == 0.0F);

    echo::audio::ReverbAdjustment room;
    room.enabled = true;
    room.mix_percent = 100;
    room.pre_delay_millis = 10;
    std::vector<float> impulse(24000 * 2, 0.0F);
    impulse[0] = 1.0F;
    impulse[1] = 1.0F;
    echo::audio::AlgorithmicReverb reverb(room, kSampleRate, 2);
    reverb.process_interleaved(impulse.data(), 24000, 2);
    // Legacy Room v1 is a persisted execution identity. These values freeze
    // its 48 kHz stereo impulse without forcing Hall/Plate into its topology.
    assert_near(tail_energy(impulse, 1), 0.110279441F, 1.0E-6F);
    assert_near(impulse[720 * 2], 0.102606565F, 1.0E-7F);
    assert_near(impulse[720 * 2 + 1], 0.0415312313F, 1.0E-7F);
    assert_near(impulse[1000 * 2], -2.75338589E-5F, 1.0E-8F);
    assert_near(impulse[2000 * 2], -0.000743589306F, 1.0E-8F);
    assert_near(impulse[5000 * 2 + 1], 3.39076105E-5F, 1.0E-8F);
    assert_near(impulse[10000 * 2 + 1], -4.40025906E-5F, 1.0E-8F);
    assert(std::abs(impulse[0]) < 0.0001F);
    assert(tail_energy(impulse, 480) > 0.0001F);
    assert(std::all_of(impulse.begin(), impulse.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) < 4.0F;
    }));

    room.size_percent = 90;
    room.decay_millis = 5000;
    reverb.update(room);
    std::vector<float> transition(4096 * 2, 0.0F);
    reverb.process_interleaved(transition.data(), 4096, 2);
    assert(std::all_of(transition.begin(), transition.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) < 4.0F;
    }));

    room.character = echo::audio::ReverbCharacter::Hall;
    reverb.update(room);
    room.character = echo::audio::ReverbCharacter::Plate;
    reverb.update(room);
    std::vector<float> character_transition(8192 * 2, 0.0F);
    reverb.process_interleaved(character_transition.data(), 8192, 2);
    assert(std::all_of(character_transition.begin(), character_transition.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) < 4.0F;
    }));

    room.character = echo::audio::ReverbCharacter::Spring;
    room.decay_millis = 2400;
    room.size_percent = 62;
    std::vector<float> spring_impulse(12000 * 2, 0.0F);
    spring_impulse[0] = 1.0F;
    spring_impulse[1] = 1.0F;
    echo::audio::AlgorithmicReverb spring(room, kSampleRate, 2);
    spring.process_interleaved(spring_impulse.data(), 12000, 2);
    assert(tail_energy(spring_impulse, 1) > 0.001F);
    assert(std::all_of(spring_impulse.begin(), spring_impulse.end(), [](float sample) {
        return std::isfinite(sample) && std::abs(sample) < 4.0F;
    }));

    reverb.reset();
    std::vector<float> silence(4096 * 2, 0.0F);
    reverb.process_interleaved(silence.data(), 4096, 2);
    assert(tail_energy(silence, 0) == 0.0F);

    bool rejected = false;
    try {
        auto invalid = room;
        invalid.decay_millis = 99;
        echo::audio::AlgorithmicReverb invalid_reverb(invalid, kSampleRate, 2);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);

    rejected = false;
    try {
        auto invalid = room;
        invalid.character = static_cast<echo::audio::ReverbCharacter>(255);
        echo::audio::AlgorithmicReverb invalid_reverb(invalid, kSampleRate, 2);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
}
