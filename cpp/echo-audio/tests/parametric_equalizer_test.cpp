#include "echo/audio/parametric_equalizer.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <stdexcept>

namespace {

constexpr std::uint32_t kSampleRate = 48000;
constexpr double kPi = 3.14159265358979323846;

double measured_rms(echo::audio::ParametricEqualizer& equalizer, double frequency) {
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

echo::audio::ParametricEqualizerAdjustment gainAt(std::size_t index, std::int16_t gain) {
    echo::audio::ParametricEqualizerAdjustment adjustment;
    adjustment.bands[index].enabled = true;
    adjustment.bands[index].gain_centibels = gain;
    return adjustment;
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

    echo::audio::ParametricEqualizer flat({}, kSampleRate, 1);
    const double flat_low = measured_rms(flat, 80.0);
    expect(flat.is_bypassed(), "zero-gain bands bypass the equalizer");

    echo::audio::ParametricEqualizer low_boost(gainAt(0, 600), kSampleRate, 1);
    expect(measured_rms(low_boost, 80.0) > flat_low * 1.65, "low shelf boosts lows");

    echo::audio::ParametricEqualizer mid_cut(gainAt(2, -600), kSampleRate, 1);
    echo::audio::ParametricEqualizer flat_mid({}, kSampleRate, 1);
    expect(
        measured_rms(mid_cut, 1000.0) < measured_rms(flat_mid, 1000.0) * 0.62,
        "bell band cuts its center"
    );

    auto notchAdjustment = echo::audio::ParametricEqualizerAdjustment{};
    notchAdjustment.bands[3] = {true, echo::audio::EqualizerFilterKind::Notch, 3000, 300, 0};
    echo::audio::ParametricEqualizer notch(notchAdjustment, kSampleRate, 1);
    echo::audio::ParametricEqualizer flat_notch({}, kSampleRate, 1);
    expect(
        measured_rms(notch, 3000.0) < measured_rms(flat_notch, 3000.0) * 0.15,
        "notch rejects its center"
    );

    echo::audio::ParametricEqualizer transitioning({}, kSampleRate, 1);
    float previous = 0.0F;
    for (std::size_t index = 0; index < kSampleRate / 4; ++index) {
        const float input = static_cast<float>(
            0.2 * std::sin(2.0 * kPi * 80.0 * static_cast<double>(index) / kSampleRate)
        );
        previous = transitioning.process_sample(input, 0);
    }
    transitioning.transition_to(gainAt(0, 600));
    float maximum_step = 0.0F;
    for (std::size_t index = 0; index < kSampleRate / 8; ++index) {
        const float input = static_cast<float>(
            0.2
            * std::sin(
                2.0 * kPi * 80.0 * static_cast<double>(index + kSampleRate / 4) / kSampleRate
            )
        );
        const float output = transitioning.process_sample(input, 0);
        maximum_step = std::max(maximum_step, std::abs(output - previous));
        previous = output;
    }
    expect(maximum_step < 0.02F, "parameter transition stays sample-continuous");

    bool rejected = false;
    try {
        auto invalid = echo::audio::ParametricEqualizerAdjustment{};
        invalid.bands[1].frequency_hertz = 0;
        flat.transition_to(invalid);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    expect(rejected, "transition rejects invalid band parameters");

    if (failures == 0) {
        std::printf("parametric equalizer test: ok\n");
        return 0;
    }
    return 1;
}
