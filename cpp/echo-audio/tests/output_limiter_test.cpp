#include "echo/audio/output_limiter.hpp"

#include <algorithm>
#include <cassert>
#include <cmath>
#include <vector>

int main() {
    constexpr std::uint32_t sample_rate = 48'000;
    constexpr std::size_t channels = 2;
    std::vector<float> samples(4'800 * channels);
    for (std::size_t frame = 0; frame < samples.size() / channels; ++frame) {
        const float value = 1.4F
                            * std::sin(
                                2.0F * 3.14159265358979323846F * 997.0F * static_cast<float>(frame)
                                / static_cast<float>(sample_rate)
                            );
        samples[frame * channels] = value;
        samples[frame * channels + 1] = value * 0.8F;
    }
    echo::audio::OutputLimiter limiter(
        {.enabled = true, .ceiling_centibels = -100, .release_millis = 100},
        sample_rate
    );
    limiter.process_interleaved(samples.data(), samples.size() / channels, channels);
    const float ceiling = std::pow(10.0F, -1.0F / 20.0F);
    float peak = 0.0F;
    for (const float sample : samples) {
        peak = std::max(peak, std::abs(sample));
    }
    assert(peak <= ceiling + 1.0E-5F);
    assert(limiter.gain_reduction_decibels() > 2.0F);

    limiter.reset();
    limiter.update({});
    std::vector<float> bypass(256 * channels, 1.2F);
    limiter.process_interleaved(bypass.data(), 256, channels);
    assert(std::abs(bypass.front() - 1.2F) < 1.0E-6F);
}
