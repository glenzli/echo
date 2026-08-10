#include "echo/audio/low_cut_filter.hpp"

#include <cmath>
#include <cstdio>
#include <numbers>

namespace {

float measured_rms(std::uint16_t cutoff_hertz, float tone_hertz) {
    constexpr std::uint32_t kSampleRate = 48'000;
    constexpr std::size_t kFrameCount = kSampleRate * 2;
    echo::audio::LowCutFilter filter(cutoff_hertz, kSampleRate, 1);
    double energy = 0.0;
    std::size_t measured = 0;
    for (std::size_t frame = 0; frame < kFrameCount; ++frame) {
        const float phase = static_cast<float>(
            2.0 * std::numbers::pi * static_cast<double>(tone_hertz) * static_cast<double>(frame)
            / static_cast<double>(kSampleRate)
        );
        const float output = filter.process_sample(std::sin(phase), 0);
        if (frame >= kSampleRate) {
            energy += static_cast<double>(output) * static_cast<double>(output);
            ++measured;
        }
    }
    return static_cast<float>(std::sqrt(energy / static_cast<double>(measured)));
}

} // namespace

int main() {
    echo::audio::LowCutFilter bypass(0, 48'000, 2);
    if (bypass.enabled() || bypass.process_sample(0.25F, 1) != 0.25F) {
        std::fprintf(stderr, "zero cutoff must be an exact bypass\n");
        return 1;
    }

    const float below_cutoff = measured_rms(80, 20.0F);
    const float above_cutoff = measured_rms(80, 1'000.0F);
    if (!(below_cutoff < 0.05F && above_cutoff > 0.65F)) {
        std::fprintf(
            stderr,
            "unexpected low-cut response: 20 Hz=%f, 1000 Hz=%f\n",
            below_cutoff,
            above_cutoff
        );
        return 1;
    }

    std::printf("low-cut filter test: ok\n");
    return 0;
}
