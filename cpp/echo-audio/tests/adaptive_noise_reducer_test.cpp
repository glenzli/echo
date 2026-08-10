#include "echo/audio/adaptive_noise_reducer.hpp"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <vector>

namespace {
constexpr std::uint32_t kSampleRate = 48000;

float maximum_step(const std::vector<float>& samples) {
    float result = 0.0F;
    for (std::size_t index = 1; index < samples.size(); ++index) {
        result = std::max(result, std::abs(samples[index] - samples[index - 1]));
    }
    return result;
}
} // namespace

int main() {
    echo::audio::AdaptiveNoiseReducer reducer(
        {.enabled = true,
         .reduction_centibels = 1200,
         .sensitivity_percent = 60,
         .smoothing_millis = 240},
        kSampleRate
    );
    std::vector<float> quiet(kSampleRate, 0.002F);
    reducer.process_interleaved(quiet.data(), quiet.size(), 1);
    assert(std::abs(quiet.back()) < 0.0012F);
    assert(maximum_step(quiet) < 0.0001F);

    std::vector<float> speech(kSampleRate / 2);
    for (std::size_t index = 0; index < speech.size(); ++index) {
        speech[index] = 0.2F
                        * std::sin(
                            2.0F * 3.14159265358979323846F * 440.0F * static_cast<float>(index)
                            / static_cast<float>(kSampleRate)
                        );
    }
    reducer.process_interleaved(speech.data(), speech.size(), 1);
    assert(std::abs(speech.back()) > 0.005F);

    reducer.update(
        {.enabled = false,
         .reduction_centibels = 1200,
         .sensitivity_percent = 60,
         .smoothing_millis = 240}
    );
    std::vector<float> transition(kSampleRate / 4, 0.002F);
    reducer.process_interleaved(transition.data(), transition.size(), 1);
    assert(maximum_step(transition) < 0.0001F);
    reducer.reset();
    assert(reducer.attenuation_decibels() < 0.001F);
}
