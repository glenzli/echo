#include "echo/audio/adjustment.hpp"

#include "echo/audio/freeze_vfx_processor.hpp"

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
    for (std::size_t index = 0; index < active_count; ++index) {
        const EffectNodeKind node = nodes[index];
        const auto value = static_cast<std::size_t>(node);
        if (value >= seen.size() || seen[value]) {
            return false;
        }
        seen[value] = true;
    }
    return true;
}

bool valid_scene_character(SceneVfxCharacter character) {
    switch (character) {
    case SceneVfxCharacter::Telephone:
    case SceneVfxCharacter::Radio:
    case SceneVfxCharacter::Intercom:
    case SceneVfxCharacter::BehindWall:
    case SceneVfxCharacter::Underwater:
        return true;
    }
    return false;
}

bool valid_delay_character(DelayVfxCharacter character) {
    return character == DelayVfxCharacter::Slapback || character == DelayVfxCharacter::Echo;
}

bool valid_modulation_character(ModulationVfxCharacter character) {
    switch (character) {
    case ModulationVfxCharacter::Chorus:
    case ModulationVfxCharacter::Flanger:
    case ModulationVfxCharacter::Phaser:
    case ModulationVfxCharacter::Tremolo:
        return true;
    }
    return false;
}

bool valid_transform_character(TransformVfxCharacter character) {
    switch (character) {
    case TransformVfxCharacter::Robot:
    case TransformVfxCharacter::Monster:
    case TransformVfxCharacter::Tiny:
    case TransformVfxCharacter::Giant:
    case TransformVfxCharacter::Ghost:
        return true;
    }
    return false;
}

bool valid_digital_degrade_character(DigitalDegradeVfxCharacter character) {
    switch (character) {
    case DigitalDegradeVfxCharacter::Bitcrusher:
    case DigitalDegradeVfxCharacter::SampleRateReduction:
    case DigitalDegradeVfxCharacter::LoFi:
        return true;
    }
    return false;
}

bool valid_drive_character(DriveVfxCharacter character) {
    switch (character) {
    case DriveVfxCharacter::SoftClip:
    case DriveVfxCharacter::Overdrive:
    case DriveVfxCharacter::Fuzz:
        return true;
    }
    return false;
}

bool valid_rotary_speed(RotaryVfxSpeed speed) {
    switch (speed) {
    case RotaryVfxSpeed::Slow:
    case RotaryVfxSpeed::Fast:
    case RotaryVfxSpeed::Brake:
        return true;
    }
    return false;
}

bool valid_creative_vfx(const CreativeVfxAdjustment& creative) {
    const SceneVfxAdjustment& scene = creative.scene;
    if (!valid_scene_character(scene.character) || scene.mix_percent > 100
        || scene.intensity_percent > 100) {
        return false;
    }

    const DelayVfxAdjustment& delay = creative.delay;
    if (!valid_delay_character(delay.character) || delay.slapback.delay_millis < 30
        || delay.slapback.delay_millis > 180 || delay.slapback.mix_percent > 100
        || delay.slapback.high_cut_hertz < 1000 || delay.slapback.high_cut_hertz > 20000
        || delay.echo.delay_millis < 80 || delay.echo.delay_millis > 2000
        || delay.echo.feedback_percent > 90 || delay.echo.mix_percent > 100
        || delay.echo.high_cut_hertz < 1000 || delay.echo.high_cut_hertz > 20000
        || delay.echo.stereo_crossfeed_percent > 100 || delay.ducking.amount_percent > 100
        || delay.ducking.attack_millis < 1 || delay.ducking.attack_millis > 200
        || delay.ducking.release_millis < 20 || delay.ducking.release_millis > 2000) {
        return false;
    }

    const ModulationVfxAdjustment& modulation = creative.modulation;
    const ChorusAdjustment& chorus = modulation.chorus;
    const FlangerAdjustment& flanger = modulation.flanger;
    const PhaserAdjustment& phaser = modulation.phaser;
    const TremoloAdjustment& tremolo = modulation.tremolo;
    if (!valid_modulation_character(modulation.character) || chorus.mix_percent > 100
        || chorus.rate_millihertz < 50 || chorus.rate_millihertz > 5000
        || chorus.minimum_delay_microseconds < 5000 || chorus.minimum_delay_microseconds > 25000
        || chorus.sweep_microseconds < 500 || chorus.sweep_microseconds > 20000
        || static_cast<std::uint32_t>(chorus.minimum_delay_microseconds) + chorus.sweep_microseconds
               > 45000
        || chorus.stereo_phase_degrees > 180 || flanger.mix_percent > 100
        || flanger.rate_millihertz < 50 || flanger.rate_millihertz > 10000
        || flanger.minimum_delay_microseconds < 100 || flanger.minimum_delay_microseconds > 5000
        || flanger.sweep_microseconds < 100 || flanger.sweep_microseconds > 10000
        || static_cast<std::uint32_t>(flanger.minimum_delay_microseconds)
                   + flanger.sweep_microseconds
               > 15000
        || flanger.feedback_percent < -90 || flanger.feedback_percent > 90
        || flanger.stereo_phase_degrees > 180 || phaser.mix_percent > 100
        || phaser.rate_millihertz < 50 || phaser.rate_millihertz > 10000
        || phaser.sweep_low_hertz < 20 || phaser.sweep_high_hertz > 20000
        || phaser.sweep_low_hertz >= phaser.sweep_high_hertz || phaser.feedback_percent < -90
        || phaser.feedback_percent > 90 || phaser.stereo_phase_degrees > 180
        || tremolo.rate_millihertz < 100 || tremolo.rate_millihertz > 20000
        || tremolo.depth_percent > 100 || tremolo.stereo_phase_degrees > 180) {
        return false;
    }

    const DigitalDegradeVfxAdjustment& digital = creative.digital_degrade;
    const DriveVfxAdjustment& drive = creative.drive;
    const RotaryVfxAdjustment& rotary = creative.rotary;
    const FreezeVfxAdjustment& freeze = creative.freeze;
    const GranularVfxAdjustment& granular = creative.granular;
    const TapeVfxParameters& tape = creative.tape;
    const PitchVfxParameters& pitch = creative.pitch;
    const AutoWahVfxParameters& auto_wah = creative.auto_wah;
    const StereoVfxParameters& stereo = creative.stereo;
    const BeatRepeatVfxParameters& beat_repeat = creative.beat_repeat;
    const double granular_pitch_ratio =
        std::exp2(static_cast<double>(granular.pitch_cents) / 1200.0);
    const double granular_required_history_millis =
        static_cast<double>(granular.lookback_millis) + static_cast<double>(granular.scatter_millis)
        + static_cast<double>(granular.grain_millis) * std::max(1.0, granular_pitch_ratio);
    return valid_transform_character(creative.transform.character)
           && creative.transform.mix_percent <= 100 && creative.transform.amount_percent <= 100
           && valid_digital_degrade_character(digital.character) && digital.mix_percent <= 100
           && digital.bitcrusher.bit_depth >= 2 && digital.bitcrusher.bit_depth <= 16
           && digital.sample_rate_reduction.target_rate_hertz >= 1000
           && digital.sample_rate_reduction.target_rate_hertz <= 24000
           && valid_drive_character(drive.character) && drive.mix_percent <= 100
           && drive.drive_centibels <= 3600 && drive.tone_hertz >= 500 && drive.tone_hertz <= 16000
           && drive.output_gain_centibels >= -2400 && drive.output_gain_centibels <= 600
           && valid_rotary_speed(rotary.speed) && rotary.mix_percent <= 100
           && rotary.motion_percent <= 100 && rotary.stereo_width_percent <= 100
           && freeze.mix_percent <= 100 && (!freeze.enabled || freeze.capture_source_millis >= 86)
           && granular.mix_percent <= 100 && granular.grain_millis >= 20
           && granular.grain_millis <= 250 && granular.density_tenths_hertz >= 10
           && granular.density_tenths_hertz <= 400 && granular.lookback_millis <= 1500
           && granular.scatter_millis <= 750 && granular.pitch_cents >= -1200
           && granular.pitch_cents <= 1200 && granular.stereo_spread_percent <= 100
           && granular_required_history_millis <= 2000.0 && tape.mix_percent <= 100
           && tape.saturation_percent <= 100 && tape.wow_flutter_percent <= 100
           && tape.dropout_percent <= 100 && pitch.mix_percent <= 100
           && pitch.pitch_semitones >= -12 && pitch.pitch_semitones <= 12
           && pitch.harmony_semitones >= -12 && pitch.harmony_semitones <= 12
           && pitch.harmony_mix_percent <= 100 && pitch.formant_colour_semitones >= -12
           && pitch.formant_colour_semitones <= 12 && auto_wah.mix_percent <= 100
           && auto_wah.sensitivity_percent <= 100 && auto_wah.minimum_frequency_hertz >= 80
           && auto_wah.maximum_frequency_hertz > auto_wah.minimum_frequency_hertz
           && auto_wah.resonance_tenths >= 5 && auto_wah.resonance_tenths <= 50
           && stereo.mix_percent <= 100 && stereo.width_percent <= 200 && stereo.pan_percent >= -100
           && stereo.pan_percent <= 100 && beat_repeat.mix_percent <= 100
           && beat_repeat.slice_millis >= 30 && beat_repeat.slice_millis <= 500
           && beat_repeat.repeat_count >= 1 && beat_repeat.repeat_count <= 4;
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
        || reverb.low_cut_hertz >= reverb.high_cut_hertz || reverb.ducking.amount_percent > 100
        || reverb.ducking.attack_millis < 1 || reverb.ducking.attack_millis > 200
        || reverb.ducking.release_millis < 20 || reverb.ducking.release_millis > 2000) {
        throw std::invalid_argument("adjustment reverb is outside the supported range");
    }
    if (authored.space.mode == SpaceMode::Convolution
        && (authored.space.convolution.import_id.empty()
            || authored.space.convolution.source_hash.empty()
            || authored.space.convolution.prepared_hash.empty()
            || (authored.space.convolution.prepared_path.empty()
                && authored.space.convolution.impulse == nullptr))) {
        throw std::invalid_argument("convolution space requires a prepared impulse response");
    }
    if (authored.space.convolution.adjustment.mix_percent > 100
        || authored.space.convolution.adjustment.wet_gain_centibels < -2400
        || authored.space.convolution.adjustment.wet_gain_centibels > 1200) {
        throw std::invalid_argument("convolution space is outside the supported range");
    }
    if (!valid_creative_vfx(authored.creative_vfx)) {
        throw std::invalid_argument("adjustment creative VFX is outside the supported range");
    }
    SpectralRepairProcessor::validate_regions(
        authored.spectral_repair,
        source_duration_millis,
        sample_rate
    );

    if (authored.creative_vfx.freeze.enabled) {
        const std::uint64_t capture_frame =
            milliseconds_to_frames(authored.creative_vfx.freeze.capture_source_millis, sample_rate);
        const std::uint64_t trim_start_frame =
            milliseconds_to_frames(authored.trim_start_millis, sample_rate);
        const std::uint64_t trim_end_frame =
            milliseconds_to_frames(authored.trim_end_millis, sample_rate);
        if (capture_frame < trim_start_frame
            || capture_frame - trim_start_frame < FreezeVfxProcessor::latency_frames()
            || capture_frame >= trim_end_frame) {
            throw std::invalid_argument(
                "freeze capture requires 4096 reachable source frames inside the trim"
            );
        }
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
    space_ = authored.space;
    space_.algorithmic = authored.reverb;
    creative_vfx_ = authored.creative_vfx;
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

const SpaceAdjustment& PreparedAdjustment::space() const {
    return space_;
}

CreativeVfxAdjustment PreparedAdjustment::creative_vfx() const {
    return creative_vfx_;
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
