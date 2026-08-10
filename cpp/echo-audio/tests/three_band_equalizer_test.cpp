#include "echo/audio/three_band_equalizer.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <stdexcept>

namespace {

constexpr std::uint32_t kSampleRate = 48000;
constexpr double kPi = 3.14159265358979323846;

double measured_rms(echo::audio::ThreeBandEqualizer& equalizer, double frequency) {
    double sum = 0.0;
    std::size_t measured = 0;
    for (std::size_t index = 0; index < kSampleRate; ++index) {
        const float input = static_cast<float>(
            0.1
            * std::sin(
                2.0 * kPi * frequency * static_cast<double>(index)
                / static_cast<double>(kSampleRate)
            )
        );
        const float output = equalizer.process_sample(input, 0);
        if (index >= kSampleRate / 4) {
            sum += static_cast<double>(output) * static_cast<double>(output);
            ++measured;
        }
    }
    return std::sqrt(sum / static_cast<double>(measured));
}

} // namespace

int main() {
    int failures = 0;
    const auto expect = [&failures](bool condition, const char* message) {
        if (!condition) {
            std::fprintf(stderr, "FAIL: %s\n", message);
            ++failures;
        }
    };

    echo::audio::ThreeBandEqualizer flat({}, kSampleRate, 1);
    const double flat_low = measured_rms(flat, 80.0);
    expect(flat.is_bypassed(), "zero gains bypass the equalizer");

    echo::audio::ThreeBandEqualizer low_boost({600, 0, 0}, kSampleRate, 1);
    expect(
        measured_rms(low_boost, 80.0) > flat_low * 1.65,
        "low shelf boosts low-frequency energy"
    );

    echo::audio::ThreeBandEqualizer mid_cut({0, -600, 0}, kSampleRate, 1);
    echo::audio::ThreeBandEqualizer flat_mid({}, kSampleRate, 1);
    expect(
        measured_rms(mid_cut, 1000.0) < measured_rms(flat_mid, 1000.0) * 0.62,
        "mid peak cuts center-frequency energy"
    );

    echo::audio::ThreeBandEqualizer high_boost({0, 0, 600}, kSampleRate, 1);
    echo::audio::ThreeBandEqualizer flat_high({}, kSampleRate, 1);
    expect(
        measured_rms(high_boost, 12000.0) > measured_rms(flat_high, 12000.0) * 1.65,
        "high shelf boosts high-frequency energy"
    );

    echo::audio::ThreeBandEqualizer transitioning({}, kSampleRate, 1);
    float previous = 0.0F;
    for (std::size_t index = 0; index < kSampleRate / 4; ++index) {
        const float input = static_cast<float>(
            0.2
            * std::sin(
                2.0 * kPi * 80.0 * static_cast<double>(index) / static_cast<double>(kSampleRate)
            )
        );
        previous = transitioning.process_sample(input, 0);
    }
    transitioning.transition_to({600, 0, 0});
    float maximum_step = 0.0F;
    for (std::size_t index = 0; index < kSampleRate / 8; ++index) {
        const float input = static_cast<float>(
            0.2
            * std::sin(
                2.0 * kPi * 80.0 * static_cast<double>(index + kSampleRate / 4)
                / static_cast<double>(kSampleRate)
            )
        );
        const float output = transitioning.process_sample(input, 0);
        maximum_step = std::max(maximum_step, std::abs(output - previous));
        previous = output;
    }
    expect(maximum_step < 0.02F, "gain transition does not introduce a sample discontinuity");

    echo::audio::ThreeBandEqualizer coalesced({}, kSampleRate, 1);
    coalesced.transition_to({600, 0, 0});
    coalesced.transition_to({-600, 0, 0});
    expect(
        measured_rms(coalesced, 80.0) < flat_low * 0.62,
        "a superseding transition converges to the newest authored target"
    );

    low_boost.reset();
    const float reset_sample = low_boost.process_sample(0.25F, 0);
    echo::audio::ThreeBandEqualizer fresh({600, 0, 0}, kSampleRate, 1);
    expect(
        std::abs(reset_sample - fresh.process_sample(0.25F, 0)) < 1.0e-6F,
        "reset clears every filter state"
    );

    bool rejected = false;
    try {
        flat.transition_to({0, 1'201, 0});
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    expect(rejected, "transition rejects gains outside the authored contract");

    if (failures == 0) {
        std::printf("three-band equalizer test: ok\n");
        return 0;
    }
    return 1;
}
