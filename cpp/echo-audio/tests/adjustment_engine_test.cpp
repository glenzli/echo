#include "echo/audio/adjustment.hpp"

#include <cassert>
#include <cmath>
#include <stdexcept>

namespace {

echo::audio::ParametricEqualizerAdjustment
legacyEqualizer(std::int16_t low, std::int16_t mid, std::int16_t high) {
    echo::audio::ParametricEqualizerAdjustment result;
    result.bands[0].gain_centibels = low;
    result.bands[2].gain_centibels = mid;
    result.bands[5].gain_centibels = high;
    return result;
}

} // namespace

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
         .restoration =
             {
                 .noise_reduction =
                     {
                         .enabled = true,
                         .reduction_centibels = 1200,
                         .sensitivity_percent = 62,
                         .smoothing_millis = 320,
                     },
                 .de_esser =
                     {
                         .enabled = true,
                         .frequency_hertz = 7200,
                         .threshold_centibels = -2800,
                         .reduction_centibels = 750,
                     },
             },
         .equalizer = legacyEqualizer(250, -175, 300),
         .compressor =
             {.enabled = true,
              .threshold_centibels = -2000,
              .ratio_tenths = 40,
              .attack_millis = 12,
              .release_millis = 160,
              .makeup_centibels = 225},
         .limiter = {.enabled = true, .ceiling_centibels = -125, .release_millis = 160}},
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
    assert(prepared.restoration().noise_reduction.reduction_centibels == 1200);
    assert(prepared.restoration().de_esser.frequency_hertz == 7200);
    assert(prepared.equalizer().bands[0].gain_centibels == 250);
    assert(prepared.equalizer().bands[2].gain_centibels == -175);
    assert(prepared.equalizer().bands[5].gain_centibels == 300);
    assert(prepared.compressor().enabled);
    assert(prepared.compressor().threshold_centibels == -2000);
    assert(prepared.limiter().enabled);
    assert(prepared.limiter().ceiling_centibels == -125);
    assert(prepared.effect_chain().front() == echo::audio::EffectNodeKind::Restoration);
    assert(prepared.effect_chain_count() == 5);
    assert(!prepared.de_hum().enabled);
    assert(!prepared.de_click().enabled);
    assert(std::abs(prepared.gain_amplitude() - 0.501187F) < 0.001F);
    assert(std::abs(prepared.envelope_at(240'000) - 1.0F) < 0.0001F);

    const echo::audio::PreparedAdjustment reduced_chain(
        {.effect_chain =
             {echo::audio::EffectNodeKind::Restoration,
              echo::audio::EffectNodeKind::Dynamics,
              echo::audio::EffectNodeKind::Master,
              echo::audio::EffectNodeKind::Equalizer,
              echo::audio::EffectNodeKind::Space,
              echo::audio::EffectNodeKind::DeHum,
              echo::audio::EffectNodeKind::DeClick,
              echo::audio::EffectNodeKind::ChannelRepair},
         .effect_chain_count = 3},
        10'000,
        48'000
    );
    assert(reduced_chain.effect_chain_count() == 3);
    assert(reduced_chain.effect_chain()[2] == echo::audio::EffectNodeKind::Master);

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
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid_chain(
            {.effect_chain =
                 {echo::audio::EffectNodeKind::Space,
                  echo::audio::EffectNodeKind::Equalizer,
                  echo::audio::EffectNodeKind::Dynamics,
                  echo::audio::EffectNodeKind::Master,
                  echo::audio::EffectNodeKind::Restoration,
                  echo::audio::EffectNodeKind::DeHum,
                  echo::audio::EffectNodeKind::DeClick,
                  echo::audio::EffectNodeKind::ChannelRepair}},
            10'000,
            48'000
        );
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);

    rejected = false;
    try {
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid_restoration(
            {.restoration = {.noise_reduction = {.smoothing_millis = 10}}},
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
            {.equalizer = legacyEqualizer(0, 1'201, 0)},
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

    rejected = false;
    try {
        [[maybe_unused]] const echo::audio::PreparedAdjustment invalid_limiter(
            {.limiter = {.ceiling_centibels = -601}},
            10'000,
            48'000
        );
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
}
