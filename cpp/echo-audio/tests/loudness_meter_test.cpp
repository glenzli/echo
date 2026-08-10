#include "echo/audio/loudness_meter.hpp"

#include <cmath>
#include <cstdio>
#include <stdexcept>
#include <vector>

namespace {

constexpr double kPi = 3.14159265358979323846;

void expect(bool condition, const char* message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

} // namespace

int main() {
    try {
        constexpr std::uint32_t sample_rate = 48'000;
        constexpr std::size_t channels = 2;
        constexpr float amplitude = 0.1F;
        std::vector<float> signal(sample_rate * channels, 0.0F);
        for (std::size_t frame = 0; frame < sample_rate; ++frame) {
            const float sample = static_cast<float>(
                amplitude
                * std::sin(
                    2.0 * kPi * 1000.0 * static_cast<double>(frame)
                    / static_cast<double>(sample_rate)
                )
            );
            signal[frame * channels] = sample;
            signal[frame * channels + 1] = sample;
        }

        echo::audio::LoudnessMeter meter(sample_rate, channels);
        meter.process_interleaved(signal.data(), sample_rate, channels);
        const echo::audio::LoudnessSnapshot measured = meter.snapshot();
        expect(
            measured.momentary_lufs > -21.0F && measured.momentary_lufs < -19.0F,
            "stereo 1 kHz reference produces calibrated momentary loudness"
        );
        expect(
            measured.sample_peak_dbfs > -20.1F && measured.sample_peak_dbfs < -19.9F,
            "sample peak reports the prepared signal amplitude"
        );

        meter.reset();
        const echo::audio::LoudnessSnapshot reset = meter.snapshot();
        expect(reset.momentary_lufs == -70.0F, "reset clears the momentary window");
        expect(reset.sample_peak_dbfs == -70.0F, "reset clears peak hold");

        bool rejected = false;
        try {
            meter.process_interleaved(signal.data(), 10, 1);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        expect(rejected, "channel layout changes are rejected");
        std::printf("loudness meter test: ok\n");
        return 0;
    } catch (const std::exception& error) {
        std::fprintf(stderr, "loudness meter test failed: %s\n", error.what());
        return 1;
    }
}
