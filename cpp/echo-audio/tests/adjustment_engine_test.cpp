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
         .gain_centibels = -600,
         .low_cut_hertz = 80,
         .equalizer =
             {.low_gain_centibels = 250, .mid_gain_centibels = -175, .high_gain_centibels = 300},
         .compressor =
             {.enabled = true,
              .threshold_centibels = -2000,
              .ratio_tenths = 40,
              .attack_millis = 12,
              .release_millis = 160,
              .makeup_centibels = 225}},
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
    assert(prepared.low_cut_hertz() == 80);
    assert(prepared.equalizer().low_gain_centibels == 250);
    assert(prepared.equalizer().mid_gain_centibels == -175);
    assert(prepared.equalizer().high_gain_centibels == 300);
    assert(prepared.compressor().enabled);
    assert(prepared.compressor().threshold_centibels == -2000);
    assert(std::abs(prepared.gain_amplitude() - 0.501187F) < 0.001F);
    assert(std::abs(prepared.envelope_at(240'000) - 1.0F) < 0.0001F);

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

    rejected = false;
    try {
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid_low_cut(
            {.low_cut_hertz = 10},
            10'000,
            48'000
        );
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);

    rejected = false;
    try {
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid_equalizer(
            {.equalizer = {.mid_gain_centibels = 1'201}},
            10'000,
            48'000
        );
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);

    rejected = false;
    try {
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid_compressor(
            {.compressor = {.ratio_tenths = 201}},
            10'000,
            48'000
        );
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
}
