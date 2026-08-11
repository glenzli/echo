#include "echo/audio/adjustment.hpp"

#include <algorithm>
#include <cmath>
#include <limits>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::int16_t kMinimumGainCentibels = -2400;
constexpr std::int16_t kMaximumGainCentibels = 1200;
constexpr std::uint16_t kMinimumLowCutHertz = 20;
constexpr std::uint16_t kMaximumLowCutHertz = 240;
constexpr std::int16_t kMinimumEqualizerGainCentibels = -1200;
constexpr std::int16_t kMaximumEqualizerGainCentibels = 1200;
constexpr std::uint16_t kMinimumEqualizerFrequencyHertz = 20;
constexpr std::uint16_t kMaximumEqualizerFrequencyHertz = 20000;
constexpr std::uint16_t kMinimumEqualizerQHundredths = 10;
constexpr std::uint16_t kMaximumEqualizerQHundredths = 2000;
constexpr std::int16_t kMinimumCompressorThresholdCentibels = -6000;
constexpr std::int16_t kMaximumCompressorThresholdCentibels = 0;
constexpr std::uint16_t kMinimumCompressorRatioTenths = 10;
constexpr std::uint16_t kMaximumCompressorRatioTenths = 200;
constexpr std::uint16_t kMinimumCompressorAttackMillis = 1;
constexpr std::uint16_t kMaximumCompressorAttackMillis = 200;
constexpr std::uint16_t kMinimumCompressorReleaseMillis = 20;
constexpr std::uint16_t kMaximumCompressorReleaseMillis = 2000;
constexpr std::int16_t kMaximumCompressorMakeupCentibels = 2400;
constexpr std::int16_t kMinimumLimiterCeilingCentibels = -600;
constexpr std::int16_t kMaximumLimiterCeilingCentibels = 0;
constexpr std::uint16_t kMinimumLimiterReleaseMillis = 20;
constexpr std::uint16_t kMaximumLimiterReleaseMillis = 1000;
constexpr float kHalfPi = 1.5707963267948966F;

bool valid_curve(FadeCurve curve) {
    switch (curve) {
    case FadeCurve::Linear:
    case FadeCurve::Smooth:
    case FadeCurve::EqualPower:
        return true;
    }
    return false;
}

bool valid_effect_chain(
    const std::array<EffectNodeKind, kEffectNodeCount>& nodes,
    std::size_t active_count
) {
    if (active_count == 0 || active_count > nodes.size()
        || nodes[active_count - 1] != EffectNodeKind::Master) {
        return false;
    }
    std::array<bool, kEffectNodeCount> seen{};
    for (const EffectNodeKind node : nodes) {
        const auto value = static_cast<std::size_t>(node);
        if (value >= seen.size() || seen[value]) {
            return false;
        }
        seen[value] = true;
    }
    return true;
}

float evaluate_curve(float progress, FadeCurve curve) {
    const float bounded = std::clamp(progress, 0.0F, 1.0F);
    switch (curve) {
    case FadeCurve::Linear:
        return bounded;
    case FadeCurve::Smooth:
        return bounded * bounded * (3.0F - 2.0F * bounded);
    case FadeCurve::EqualPower:
        return std::sin(bounded * kHalfPi);
    }
    return bounded;
}

std::uint64_t milliseconds_to_frames(std::uint64_t millis, std::uint32_t sample_rate) {
    if (sample_rate == 0) {
        throw std::invalid_argument("adjustment sample rate must be positive");
    }
    if (millis > std::numeric_limits<std::uint64_t>::max() / sample_rate) {
        throw std::invalid_argument("adjustment time exceeds the supported range");
    }
    return millis * sample_rate / 1000;
}

} // namespace

PreparedAdjustment::PreparedAdjustment(
    PlaybackAdjustment authored,
    std::uint64_t source_duration_millis,
    std::uint32_t sample_rate
) {
    if (source_duration_millis == 0) {
        throw std::invalid_argument("adjustment requires a known source duration");
    }
    if (authored.trim_end_millis == 0) {
        authored.trim_end_millis = source_duration_millis;
    }
    if (authored.trim_start_millis >= authored.trim_end_millis
        || authored.trim_end_millis > source_duration_millis) {
        throw std::invalid_argument("adjustment trim range is outside the source");
    }
    const std::uint64_t selected_millis = authored.trim_end_millis - authored.trim_start_millis;
    if (authored.fade_in_millis + authored.fade_out_millis > selected_millis) {
        throw std::invalid_argument("adjustment fades overlap");
    }
    if (authored.gain_centibels < kMinimumGainCentibels
        || authored.gain_centibels > kMaximumGainCentibels) {
        throw std::invalid_argument("adjustment gain is outside the supported range");
    }
    if (!valid_curve(authored.fade_in_curve) || !valid_curve(authored.fade_out_curve)) {
        throw std::invalid_argument("adjustment fade curve is outside the supported range");
    }
    if (authored.low_cut_hertz != 0
        && (authored.low_cut_hertz < kMinimumLowCutHertz
            || authored.low_cut_hertz > kMaximumLowCutHertz)) {
        throw std::invalid_argument("adjustment low cut is outside the supported range");
    }
    if (!valid_effect_chain(authored.effect_chain, authored.effect_chain_count)) {
        throw std::invalid_argument(
            "adjustment effect chain must contain unique singleton nodes with master last"
        );
    }
    const DePlosiveAdjustment de_plosive = authored.restoration.de_plosive;
    if (de_plosive.frequency_hertz < 80 || de_plosive.frequency_hertz > 240
        || de_plosive.sensitivity_percent > 100 || de_plosive.reduction_centibels > 1800
        || de_plosive.release_millis < 40 || de_plosive.release_millis > 500) {
        throw std::invalid_argument("adjustment de-plosive is outside the supported range");
    }
    const NoiseReductionAdjustment noise_reduction = authored.restoration.noise_reduction;
    if (noise_reduction.reduction_centibels > 2400 || noise_reduction.sensitivity_percent > 100
        || noise_reduction.smoothing_millis < 20 || noise_reduction.smoothing_millis > 1000) {
        throw std::invalid_argument("adjustment noise reduction is outside the supported range");
    }
    const DeEsserAdjustment de_esser = authored.restoration.de_esser;
    if (de_esser.frequency_hertz < 3000 || de_esser.frequency_hertz > 12000
        || de_esser.threshold_centibels < -6000 || de_esser.threshold_centibels > 0
        || de_esser.reduction_centibels > 1800) {
        throw std::invalid_argument("adjustment de-esser is outside the supported range");
    }
    const DeHumAdjustment de_hum = authored.de_hum;
    if ((de_hum.fundamental_hertz != 50 && de_hum.fundamental_hertz != 60)
        || de_hum.harmonic_count < 1 || de_hum.harmonic_count > 8 || de_hum.quality_tenths < 50
        || de_hum.quality_tenths > 1000 || de_hum.depth_centibels > 4800) {
        throw std::invalid_argument("adjustment de-hum is outside the supported range");
    }
    const DeClickAdjustment de_click = authored.de_click;
    if (de_click.sensitivity_percent > 100 || de_click.maximum_click_microseconds < 50
        || de_click.maximum_click_microseconds > 2000 || de_click.repair_percent > 100) {
        throw std::invalid_argument("adjustment de-click is outside the supported range");
    }
    if (authored.channel_repair.balance_percent < -100
        || authored.channel_repair.balance_percent > 100) {
        throw std::invalid_argument("adjustment channel repair is outside the supported range");
    }
    for (const ParametricEqualizerBand& band : authored.equalizer.bands) {
        if (band.gain_centibels < kMinimumEqualizerGainCentibels
            || band.gain_centibels > kMaximumEqualizerGainCentibels
            || band.frequency_hertz < kMinimumEqualizerFrequencyHertz
            || band.frequency_hertz > kMaximumEqualizerFrequencyHertz
            || band.q_hundredths < kMinimumEqualizerQHundredths
            || band.q_hundredths > kMaximumEqualizerQHundredths) {
            throw std::invalid_argument("adjustment equalizer band is outside the supported range");
        }
    }
    const CompressorAdjustment compressor = authored.compressor;
    if (compressor.threshold_centibels < kMinimumCompressorThresholdCentibels
        || compressor.threshold_centibels > kMaximumCompressorThresholdCentibels
        || compressor.ratio_tenths < kMinimumCompressorRatioTenths
        || compressor.ratio_tenths > kMaximumCompressorRatioTenths
        || compressor.attack_millis < kMinimumCompressorAttackMillis
        || compressor.attack_millis > kMaximumCompressorAttackMillis
        || compressor.release_millis < kMinimumCompressorReleaseMillis
        || compressor.release_millis > kMaximumCompressorReleaseMillis
        || compressor.makeup_centibels < 0
        || compressor.makeup_centibels > kMaximumCompressorMakeupCentibels) {
        throw std::invalid_argument("adjustment compressor is outside the supported range");
    }
    const LimiterAdjustment limiter = authored.limiter;
    if (limiter.ceiling_centibels < kMinimumLimiterCeilingCentibels
        || limiter.ceiling_centibels > kMaximumLimiterCeilingCentibels
        || limiter.release_millis < kMinimumLimiterReleaseMillis
        || limiter.release_millis > kMaximumLimiterReleaseMillis) {
        throw std::invalid_argument("adjustment limiter is outside the supported range");
    }
    const ReverbAdjustment reverb = authored.reverb;
    if (reverb.mix_percent > 100 || reverb.pre_delay_millis > 200 || reverb.decay_millis < 100
        || reverb.decay_millis > 12000 || reverb.size_percent < 10 || reverb.size_percent > 100
        || reverb.damping_percent > 100 || reverb.low_cut_hertz < 20 || reverb.low_cut_hertz > 1000
        || reverb.high_cut_hertz < 1000 || reverb.high_cut_hertz > 20000
        || reverb.low_cut_hertz >= reverb.high_cut_hertz) {
        throw std::invalid_argument("adjustment reverb is outside the supported range");
    }

    trim_start_millis_ = authored.trim_start_millis;
    trim_end_millis_ = authored.trim_end_millis;
    start_frame_ = milliseconds_to_frames(trim_start_millis_, sample_rate);
    end_frame_ = milliseconds_to_frames(trim_end_millis_, sample_rate);
    fade_in_frames_ = milliseconds_to_frames(authored.fade_in_millis, sample_rate);
    fade_out_frames_ = milliseconds_to_frames(authored.fade_out_millis, sample_rate);
    fade_in_curve_ = authored.fade_in_curve;
    fade_out_curve_ = authored.fade_out_curve;
    gain_amplitude_ = std::pow(10.0F, static_cast<float>(authored.gain_centibels) / 2000.0F);
    low_cut_hertz_ = authored.low_cut_hertz;
    restoration_ = authored.restoration;
    de_hum_ = authored.de_hum;
    de_click_ = authored.de_click;
    channel_repair_ = authored.channel_repair;
    equalizer_ = authored.equalizer;
    compressor_ = authored.compressor;
    reverb_ = authored.reverb;
    limiter_ = authored.limiter;
    effect_chain_ = authored.effect_chain;
    effect_chain_count_ = authored.effect_chain_count;
}

std::uint64_t PreparedAdjustment::start_frame() const {
    return start_frame_;
}

std::uint64_t PreparedAdjustment::end_frame() const {
    return end_frame_;
}

std::uint64_t PreparedAdjustment::trim_start_millis() const {
    return trim_start_millis_;
}

std::uint64_t PreparedAdjustment::trim_end_millis() const {
    return trim_end_millis_;
}

std::uint16_t PreparedAdjustment::low_cut_hertz() const {
    return low_cut_hertz_;
}

RestorationAdjustment PreparedAdjustment::restoration() const {
    return restoration_;
}

DeHumAdjustment PreparedAdjustment::de_hum() const {
    return de_hum_;
}

DeClickAdjustment PreparedAdjustment::de_click() const {
    return de_click_;
}

ChannelRepairAdjustment PreparedAdjustment::channel_repair() const {
    return channel_repair_;
}

ParametricEqualizerAdjustment PreparedAdjustment::equalizer() const {
    return equalizer_;
}

CompressorAdjustment PreparedAdjustment::compressor() const {
    return compressor_;
}

ReverbAdjustment PreparedAdjustment::reverb() const {
    return reverb_;
}

LimiterAdjustment PreparedAdjustment::limiter() const {
    return limiter_;
}

std::array<EffectNodeKind, kEffectNodeCount> PreparedAdjustment::effect_chain() const {
    return effect_chain_;
}

std::size_t PreparedAdjustment::effect_chain_count() const {
    return effect_chain_count_;
}

std::uint64_t PreparedAdjustment::clamp_seek_millis(std::uint64_t millis) const {
    return std::clamp(millis, trim_start_millis_, trim_end_millis_);
}

float PreparedAdjustment::amplitude_at(std::uint64_t source_frame) const {
    return gain_amplitude_ * envelope_at(source_frame);
}

float PreparedAdjustment::gain_amplitude() const {
    return gain_amplitude_;
}

float PreparedAdjustment::envelope_at(std::uint64_t source_frame) const {
    if (source_frame < start_frame_ || source_frame >= end_frame_) {
        return 0.0F;
    }

    float envelope = 1.0F;
    if (fade_in_frames_ > 0 && source_frame < start_frame_ + fade_in_frames_) {
        const float progress =
            static_cast<float>(source_frame - start_frame_) / static_cast<float>(fade_in_frames_);
        envelope = evaluate_curve(progress, fade_in_curve_);
    }
    if (fade_out_frames_ > 0 && source_frame >= end_frame_ - fade_out_frames_) {
        const float progress =
            static_cast<float>(end_frame_ - source_frame) / static_cast<float>(fade_out_frames_);
        const float fade_out = evaluate_curve(progress, fade_out_curve_);
        envelope = std::min(envelope, fade_out);
    }
    return envelope;
}

} // namespace echo::audio
