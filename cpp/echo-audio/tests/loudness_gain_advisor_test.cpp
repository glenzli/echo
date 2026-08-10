#include "echo/audio/loudness_gain_advisor.hpp"

#include <cassert>
#include <cmath>
#include <cstdio>
#include <stdexcept>

namespace {

void expect_near(float actual, float expected) {
    assert(std::abs(actual - expected) < 0.01F);
}

} // namespace

int main() {
    try {
        using echo::audio::LoudnessGainAdvisor;
        using echo::audio::LoudnessGainRequest;

        const auto reachable = LoudnessGainAdvisor::advise({
            .integrated_lufs = -20.0F,
            .true_peak_dbtp = -6.0F,
            .target_lufs = -16.0F,
            .true_peak_ceiling_dbtp = -1.0F,
            .current_gain_centibels = 0,
        });
        assert(reachable.available);
        assert(reachable.gain_delta_centibels == 400);
        assert(reachable.resulting_gain_centibels == 400);
        assert(reachable.target_reached);
        assert(!reachable.peak_constrained);
        expect_near(reachable.estimated_integrated_lufs, -16.0F);
        expect_near(reachable.estimated_true_peak_dbtp, -2.0F);

        const auto peak_limited = LoudnessGainAdvisor::advise({
            .integrated_lufs = -20.0F,
            .true_peak_dbtp = -2.53F,
            .target_lufs = -14.0F,
            .true_peak_ceiling_dbtp = -1.0F,
            .current_gain_centibels = 0,
        });
        assert(peak_limited.gain_delta_centibels == 150);
        assert(peak_limited.peak_constrained);
        assert(!peak_limited.target_reached);
        assert(peak_limited.estimated_true_peak_dbtp <= -1.0F);

        const auto attenuate = LoudnessGainAdvisor::advise({
            .integrated_lufs = -16.0F,
            .true_peak_dbtp = -1.5F,
            .target_lufs = -23.0F,
            .true_peak_ceiling_dbtp = -1.0F,
            .current_gain_centibels = 300,
        });
        assert(attenuate.gain_delta_centibels == -700);
        assert(attenuate.resulting_gain_centibels == -400);
        assert(attenuate.target_reached);

        const auto gain_bounded = LoudnessGainAdvisor::advise({
            .integrated_lufs = -22.0F,
            .true_peak_dbtp = -10.0F,
            .target_lufs = -16.0F,
            .true_peak_ceiling_dbtp = -1.0F,
            .current_gain_centibels = 1150,
        });
        assert(gain_bounded.gain_delta_centibels == 50);
        assert(gain_bounded.resulting_gain_centibels == 1200);
        assert(gain_bounded.gain_range_constrained);
        assert(!gain_bounded.target_reached);

        const auto silent = LoudnessGainAdvisor::advise({});
        assert(!silent.available);
        assert(silent.gain_delta_centibels == 0);

        std::printf("loudness gain advisor test: ok\n");
        return 0;
    } catch (const std::exception& error) {
        std::fprintf(stderr, "loudness gain advisor test failed: %s\n", error.what());
        return 1;
    }
}
