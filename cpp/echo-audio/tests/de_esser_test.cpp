#include "echo/audio/de_esser.hpp"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <vector>

namespace {
constexpr std::uint32_t kSampleRate = 48000;

float rms(const std::vector<float>& samples) {
    double sum = 0.0;
    for (float sample : samples) {
        sum += static_cast<double>(sample) * sample;
    }
    return static_cast<float>(std::sqrt(sum / static_cast<double>(samples.size())));
}

std::vector<float> tone(float frequency) {
    std::vector<float> result(kSampleRate);
    for (std::size_t index = 0; index < result.size(); ++index) {
        result[index] = 0.3F
                        * std::sin(
                            2.0F * 3.14159265358979323846F * frequency * static_cast<float>(index)
                            / static_cast<float>(kSampleRate)
                        );
    }
    return result;
}
} // namespace

int main() {
    echo::audio::DeEsser de_esser(
        {.enabled = true,
         .frequency_hertz = 6000,
         .threshold_centibels = -3000,
         .reduction_centibels = 900},
        kSampleRate,
        1
    );
    auto sibilance = tone(9000.0F);
    const float dry_sibilance = rms(sibilance);
    de_esser.process_interleaved(sibilance.data(), sibilance.size(), 1);
    assert(rms(sibilance) < dry_sibilance * 0.75F);
    assert(de_esser.attenuation_decibels() > 1.0F);

    de_esser.reset();
    auto body = tone(500.0F);
    const float dry_body = rms(body);
    de_esser.process_interleaved(body.data(), body.size(), 1);
    assert(rms(body) > dry_body * 0.95F);

    de_esser.update(
        {.enabled = false,
         .frequency_hertz = 6000,
         .threshold_centibels = -3000,
         .reduction_centibels = 900}
    );
    auto transition = tone(9000.0F);
    de_esser.process_interleaved(transition.data(), transition.size(), 1);
    assert(std::all_of(transition.begin(), transition.end(), [](float sample) {
        return std::isfinite(sample);
    }));
}
