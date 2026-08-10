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
}
