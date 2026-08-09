#include "echo/audio/adjustment.hpp"

#include <cassert>
#include <cmath>
#include <stdexcept>

int main() {
    const echo::audio::PreparedAdjustment identity({}, 10'000, 48'000);
    assert(identity.start_frame() == 0);
    assert(identity.end_frame() == 480'000);
    assert(std::abs(identity.amplitude_at(240'000) - 1.0F) < 0.0001F);

    const echo::audio::PreparedAdjustment prepared(
        {.trim_start_millis = 1'000,
         .trim_end_millis = 9'000,
         .fade_in_millis = 500,
         .fade_out_millis = 1'000,
         .fade_in_curve = echo::audio::FadeCurve::Smooth,
         .fade_out_curve = echo::audio::FadeCurve::EqualPower,
         .gain_centibels = -600},
        10'000,
        48'000
    );
    assert(prepared.start_frame() == 48'000);
    assert(prepared.end_frame() == 432'000);
    assert(prepared.clamp_seek_millis(0) == 1'000);
    assert(prepared.clamp_seek_millis(10'000) == 9'000);
    assert(prepared.amplitude_at(47'999) == 0.0F);
    assert(prepared.amplitude_at(48'000) == 0.0F);
    assert(prepared.amplitude_at(432'000) == 0.0F);
    assert(std::abs(prepared.amplitude_at(240'000) - 0.501187F) < 0.001F);
    assert(prepared.amplitude_at(60'000) < prepared.amplitude_at(72'000));
    assert(prepared.amplitude_at(420'000) < prepared.amplitude_at(384'000));

    const echo::audio::PreparedAdjustment linear(
        {.trim_end_millis = 1'000, .fade_in_millis = 1'000},
        1'000,
        48'000
    );
    const echo::audio::PreparedAdjustment smooth(
        {.trim_end_millis = 1'000,
         .fade_in_millis = 1'000,
         .fade_in_curve = echo::audio::FadeCurve::Smooth},
        1'000,
        48'000
    );
    const echo::audio::PreparedAdjustment equal_power(
        {.trim_end_millis = 1'000,
         .fade_in_millis = 1'000,
         .fade_in_curve = echo::audio::FadeCurve::EqualPower},
        1'000,
        48'000
    );
    assert(std::abs(linear.amplitude_at(12'000) - 0.25F) < 0.0001F);
    assert(smooth.amplitude_at(12'000) < linear.amplitude_at(12'000));
    assert(equal_power.amplitude_at(12'000) > linear.amplitude_at(12'000));

    bool rejected = false;
    try {
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid(
            {.trim_start_millis = 1'000, .trim_end_millis = 500},
            10'000,
            48'000
        );
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
}
