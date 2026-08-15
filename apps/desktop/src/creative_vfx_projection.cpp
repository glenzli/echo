#include "creative_vfx_projection.hpp"

#include <QJsonDocument>
#include <QJsonObject>
#include <QStringList>

#include <algorithm>
#include <cmath>
#include <limits>

namespace {

QVariant
field(const QVariantMap& value, const char* camel, const char* snake, const QVariant& fallback) {
    const QString camel_key = QString::fromLatin1(camel);
    if (value.contains(camel_key)) {
        return value.value(camel_key);
    }
    return value.value(QString::fromLatin1(snake), fallback);
}

std::optional<int> character(const QVariant& value, const QStringList& names) {
    bool integer_ok = false;
    const int integer = value.toInt(&integer_ok);
    if (integer_ok && integer >= 0 && integer < names.size()) {
        return integer;
    }
    const qsizetype named = names.indexOf(value.toString());
    return named >= 0 ? std::optional<int>(static_cast<int>(named)) : std::nullopt;
}

QVariantMap nested(const QVariantMap& value, const char* name) {
    return value.value(QString::fromLatin1(name)).toMap();
}

QVariantMap familyHeader(int kind, bool enabled) {
    return {
        {QStringLiteral("character"), kind},
        {QStringLiteral("enabled"), enabled},
    };
}

} // namespace

std::optional<echo::audio::CreativeVfxAdjustment>
CreativeVfxProjection::fromQml(const QVariantMap& value) {
    const QVariantMap scene = nested(value, "scene");
    const auto scene_character = character(
        scene.value(QStringLiteral("character"), 0),
        {"telephone", "radio", "intercom", "behind_wall", "underwater"}
    );
    const int scene_mix = field(scene, "mixPercent", "mix_percent", 100).toInt();
    const int scene_intensity = field(scene, "intensityPercent", "intensity_percent", 50).toInt();

    const QVariantMap delay = nested(value, "delay");
    const auto delay_character =
        character(delay.value(QStringLiteral("character"), 0), {"slapback", "echo"});
    const QVariantMap slapback = nested(delay, "slapback");
    const int slapback_time = field(slapback, "delayMillis", "delay_millis", 90).toInt();
    const int slapback_mix = field(slapback, "mixPercent", "mix_percent", 22).toInt();
    const int slapback_cut = field(slapback, "highCutHertz", "high_cut_hertz", 7000).toInt();
    const QVariantMap echo = nested(delay, "echo");
    const int echo_time = field(echo, "delayMillis", "delay_millis", 375).toInt();
    const int echo_feedback = field(echo, "feedbackPercent", "feedback_percent", 36).toInt();
    const int echo_mix = field(echo, "mixPercent", "mix_percent", 28).toInt();
    const int echo_cut = field(echo, "highCutHertz", "high_cut_hertz", 6500).toInt();
    const int echo_cross =
        field(echo, "stereoCrossfeedPercent", "stereo_crossfeed_percent", 70).toInt();
    const QVariantMap ducking = nested(delay, "ducking");
    const int ducking_amount = field(ducking, "amountPercent", "amount_percent", 65).toInt();
    const int ducking_attack = field(ducking, "attackMillis", "attack_millis", 10).toInt();
    const int ducking_release = field(ducking, "releaseMillis", "release_millis", 250).toInt();

    const QVariantMap modulation = nested(value, "modulation");
    const auto modulation_character = character(
        modulation.value(QStringLiteral("character"), 0),
        {"chorus", "flanger", "phaser", "tremolo"}
    );
    const QVariantMap chorus = nested(modulation, "chorus");
    const int chorus_mix = field(chorus, "mixPercent", "mix_percent", 35).toInt();
    const int chorus_rate = field(chorus, "rateMillihertz", "rate_millihertz", 800).toInt();
    const int chorus_minimum =
        field(chorus, "minimumDelayMicroseconds", "minimum_delay_microseconds", 8000).toInt();
    const int chorus_sweep =
        field(chorus, "sweepMicroseconds", "sweep_microseconds", 10000).toInt();
    const int chorus_phase =
        field(chorus, "stereoPhaseDegrees", "stereo_phase_degrees", 90).toInt();
    const QVariantMap flanger = nested(modulation, "flanger");
    const int flanger_mix = field(flanger, "mixPercent", "mix_percent", 50).toInt();
    const int flanger_rate = field(flanger, "rateMillihertz", "rate_millihertz", 250).toInt();
    const int flanger_minimum =
        field(flanger, "minimumDelayMicroseconds", "minimum_delay_microseconds", 200).toInt();
    const int flanger_sweep =
        field(flanger, "sweepMicroseconds", "sweep_microseconds", 3500).toInt();
    const int flanger_feedback = field(flanger, "feedbackPercent", "feedback_percent", 35).toInt();
    const int flanger_phase =
        field(flanger, "stereoPhaseDegrees", "stereo_phase_degrees", 180).toInt();
    const QVariantMap phaser = nested(modulation, "phaser");
    const int phaser_mix = field(phaser, "mixPercent", "mix_percent", 50).toInt();
    const int phaser_rate = field(phaser, "rateMillihertz", "rate_millihertz", 350).toInt();
    const int phaser_low = field(phaser, "sweepLowHertz", "sweep_low_hertz", 300).toInt();
    const int phaser_high = field(phaser, "sweepHighHertz", "sweep_high_hertz", 2500).toInt();
    const int phaser_feedback = field(phaser, "feedbackPercent", "feedback_percent", 25).toInt();
    const int phaser_phase =
        field(phaser, "stereoPhaseDegrees", "stereo_phase_degrees", 90).toInt();
    const QVariantMap tremolo = nested(modulation, "tremolo");
    const int tremolo_rate = field(tremolo, "rateMillihertz", "rate_millihertz", 4000).toInt();
    const int tremolo_depth = field(tremolo, "depthPercent", "depth_percent", 60).toInt();
    const int tremolo_phase =
        field(tremolo, "stereoPhaseDegrees", "stereo_phase_degrees", 0).toInt();

    const QVariantMap transform = nested(value, "transform");
    const auto transform_character = character(
        transform.value(QStringLiteral("character"), 0),
        {"robot", "monster", "tiny", "giant", "ghost"}
    );
    const int transform_mix = field(transform, "mixPercent", "mix_percent", 100).toInt();
    const int transform_amount = field(transform, "amountPercent", "amount_percent", 50).toInt();

    const QVariantMap digital_degrade = nested(value, "digital_degrade").isEmpty()
                                            ? nested(value, "digitalDegrade")
                                            : nested(value, "digital_degrade");
    const auto digital_degrade_character = character(
        digital_degrade.value(QStringLiteral("character"), 0),
        {"bitcrusher", "sample_rate_reduction", "lo_fi"}
    );
    const int digital_mix = field(digital_degrade, "mixPercent", "mix_percent", 100).toInt();
    const QVariantMap bitcrusher = nested(digital_degrade, "bitcrusher");
    const int bit_depth = field(bitcrusher, "bitDepth", "bit_depth", 8).toInt();
    const QVariantMap sample_rate_reduction =
        nested(digital_degrade, "sample_rate_reduction").isEmpty()
            ? nested(digital_degrade, "sampleRateReduction")
            : nested(digital_degrade, "sample_rate_reduction");
    const int target_rate =
        field(sample_rate_reduction, "targetRateHertz", "target_rate_hertz", 8000).toInt();

    const QVariantMap drive = nested(value, "drive");
    const auto drive_character =
        character(drive.value(QStringLiteral("character"), 0), {"soft_clip", "overdrive", "fuzz"});
    const int drive_mix = field(drive, "mixPercent", "mix_percent", 100).toInt();
    const int drive_amount = field(drive, "driveCentibels", "drive_centibels", 1200).toInt();
    const int drive_tone = field(drive, "toneHertz", "tone_hertz", 8000).toInt();
    const int drive_output =
        field(drive, "outputGainCentibels", "output_gain_centibels", -300).toInt();

    const QVariantMap rotary = nested(value, "rotary");
    const auto rotary_speed =
        character(rotary.value(QStringLiteral("speed"), 0), {"slow", "fast", "brake"});
    const int rotary_mix = field(rotary, "mixPercent", "mix_percent", 55).toInt();
    const int rotary_motion = field(rotary, "motionPercent", "motion_percent", 65).toInt();
    const int rotary_width =
        field(rotary, "stereoWidthPercent", "stereo_width_percent", 80).toInt();

    const QVariantMap freeze = nested(value, "freeze");
    const int freeze_mix = field(freeze, "mixPercent", "mix_percent", 70).toInt();
    bool freeze_anchor_ok = false;
    const qlonglong freeze_anchor =
        field(freeze, "captureSourceMillis", "capture_source_millis", 100)
            .toLongLong(&freeze_anchor_ok);

    const QVariantMap granular = nested(value, "granular");
    const int granular_mix = field(granular, "mixPercent", "mix_percent", 45).toInt();
    const int grain_millis = field(granular, "grainMillis", "grain_millis", 80).toInt();
    const int density = field(granular, "densityTenthsHertz", "density_tenths_hertz", 120).toInt();
    const int lookback = field(granular, "lookbackMillis", "lookback_millis", 250).toInt();
    const int scatter = field(granular, "scatterMillis", "scatter_millis", 120).toInt();
    const int pitch = field(granular, "pitchCents", "pitch_cents", 0).toInt();
    const int spread = field(granular, "stereoSpreadPercent", "stereo_spread_percent", 50).toInt();
    bool seed_ok = false;
    const qulonglong seed =
        field(granular, "randomSeed", "random_seed", 0x4543484FU).toULongLong(&seed_ok);
    const double pitch_ratio = std::exp2(static_cast<double>(pitch) / 1200.0);
    const double required_history =
        static_cast<double>(lookback + scatter)
        + static_cast<double>(grain_millis) * std::max(1.0, pitch_ratio);
    const QVariantMap tape = nested(value, "tape");
    const int tape_mix = field(tape, "mixPercent", "mix_percent", 55).toInt();
    const int tape_saturation = field(tape, "saturationPercent", "saturation_percent", 25).toInt();
    const int tape_motion = field(tape, "wowFlutterPercent", "wow_flutter_percent", 30).toInt();
    const int tape_dropout = field(tape, "dropoutPercent", "dropout_percent", 0).toInt();
    const QVariantMap pitch_vfx = nested(value, "pitch");
    const int pitch_mix = field(pitch_vfx, "mixPercent", "mix_percent", 100).toInt();
    const int pitch_semitones = field(pitch_vfx, "pitchSemitones", "pitch_semitones", 0).toInt();
    const int harmony_semitones =
        field(pitch_vfx, "harmonySemitones", "harmony_semitones", 7).toInt();
    const int harmony_mix =
        field(pitch_vfx, "harmonyMixPercent", "harmony_mix_percent", 35).toInt();
    const int formant_colour =
        field(pitch_vfx, "formantColourSemitones", "formant_colour_semitones", 0).toInt();
    const QVariantMap auto_wah =
        nested(value, "auto_wah").isEmpty() ? nested(value, "autoWah") : nested(value, "auto_wah");
    const int wah_mix = field(auto_wah, "mixPercent", "mix_percent", 70).toInt();
    const int wah_sensitivity =
        field(auto_wah, "sensitivityPercent", "sensitivity_percent", 55).toInt();
    const int wah_minimum =
        field(auto_wah, "minimumFrequencyHertz", "minimum_frequency_hertz", 280).toInt();
    const int wah_maximum =
        field(auto_wah, "maximumFrequencyHertz", "maximum_frequency_hertz", 2800).toInt();
    const int wah_resonance = field(auto_wah, "resonanceTenths", "resonance_tenths", 18).toInt();

    if (!scene_character || scene_mix < 0 || scene_mix > 100 || scene_intensity < 0
        || scene_intensity > 100 || !delay_character || slapback_time < 30 || slapback_time > 180
        || slapback_mix < 0 || slapback_mix > 100 || slapback_cut < 1000 || slapback_cut > 20000
        || echo_time < 80 || echo_time > 2000 || echo_feedback < 0 || echo_feedback > 90
        || echo_mix < 0 || echo_mix > 100 || echo_cut < 1000 || echo_cut > 20000 || echo_cross < 0
        || echo_cross > 100 || ducking_amount < 0 || ducking_amount > 100 || ducking_attack < 1
        || ducking_attack > 200 || ducking_release < 20 || ducking_release > 2000
        || !modulation_character || chorus_mix < 0 || chorus_mix > 100 || chorus_rate < 50
        || chorus_rate > 5000 || chorus_minimum < 5000 || chorus_minimum > 25000
        || chorus_sweep < 500 || chorus_sweep > 20000 || chorus_minimum + chorus_sweep > 45000
        || chorus_phase < 0 || chorus_phase > 180 || flanger_mix < 0 || flanger_mix > 100
        || flanger_rate < 50 || flanger_rate > 10000 || flanger_minimum < 100
        || flanger_minimum > 5000 || flanger_sweep < 100 || flanger_sweep > 10000
        || flanger_minimum + flanger_sweep > 15000 || flanger_feedback < -90
        || flanger_feedback > 90 || flanger_phase < 0 || flanger_phase > 180 || phaser_mix < 0
        || phaser_mix > 100 || phaser_rate < 50 || phaser_rate > 10000 || phaser_low < 20
        || phaser_high > 20000 || phaser_low >= phaser_high || phaser_feedback < -90
        || phaser_feedback > 90 || phaser_phase < 0 || phaser_phase > 180 || tremolo_rate < 100
        || tremolo_rate > 20000 || tremolo_depth < 0 || tremolo_depth > 100 || tremolo_phase < 0
        || tremolo_phase > 180 || !transform_character || transform_mix < 0 || transform_mix > 100
        || transform_amount < 0 || transform_amount > 100 || !digital_degrade_character
        || digital_mix < 0 || digital_mix > 100 || bit_depth < 2 || bit_depth > 16
        || target_rate < 1000 || target_rate > 24000 || !drive_character || drive_mix < 0
        || drive_mix > 100 || drive_amount < 0 || drive_amount > 3600 || drive_tone < 500
        || drive_tone > 16000 || drive_output < -2400 || drive_output > 600 || !rotary_speed
        || rotary_mix < 0 || rotary_mix > 100 || rotary_motion < 0 || rotary_motion > 100
        || rotary_width < 0 || rotary_width > 100 || freeze_mix < 0 || freeze_mix > 100
        || !freeze_anchor_ok || freeze_anchor < 0
        || (freeze.value(QStringLiteral("enabled"), false).toBool() && freeze_anchor < 86)
        || granular_mix < 0 || granular_mix > 100 || grain_millis < 20 || grain_millis > 250
        || density < 10 || density > 400 || lookback < 0 || lookback > 1500 || scatter < 0
        || scatter > 750 || pitch < -1200 || pitch > 1200 || spread < 0 || spread > 100 || !seed_ok
        || seed > std::numeric_limits<std::uint32_t>::max() || required_history > 2000.0) {
        return std::nullopt;
    }
    if (tape_mix < 0 || tape_mix > 100 || tape_saturation < 0 || tape_saturation > 100
        || tape_motion < 0 || tape_motion > 100 || tape_dropout < 0 || tape_dropout > 100
        || pitch_mix < 0 || pitch_mix > 100 || pitch_semitones < -12 || pitch_semitones > 12
        || harmony_semitones < -12 || harmony_semitones > 12 || harmony_mix < 0 || harmony_mix > 100
        || formant_colour < -12 || formant_colour > 12 || wah_mix < 0 || wah_mix > 100
        || wah_sensitivity < 0 || wah_sensitivity > 100 || wah_minimum < 80
        || wah_maximum <= wah_minimum || wah_resonance < 5 || wah_resonance > 50) {
        return std::nullopt;
    }

    return echo::audio::CreativeVfxAdjustment{
        .scene =
            {
                .character = static_cast<echo::audio::SceneVfxCharacter>(*scene_character),
                .enabled = scene.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(scene_mix),
                .intensity_percent = static_cast<std::uint8_t>(scene_intensity),
            },
        .delay =
            {
                .character = static_cast<echo::audio::DelayVfxCharacter>(*delay_character),
                .enabled = delay.value(QStringLiteral("enabled"), false).toBool(),
                .slapback =
                    {
                        .delay_millis = static_cast<std::uint16_t>(slapback_time),
                        .mix_percent = static_cast<std::uint8_t>(slapback_mix),
                        .high_cut_hertz = static_cast<std::uint16_t>(slapback_cut),
                    },
                .echo =
                    {
                        .delay_millis = static_cast<std::uint16_t>(echo_time),
                        .feedback_percent = static_cast<std::uint8_t>(echo_feedback),
                        .mix_percent = static_cast<std::uint8_t>(echo_mix),
                        .high_cut_hertz = static_cast<std::uint16_t>(echo_cut),
                        .stereo_crossfeed_percent = static_cast<std::uint8_t>(echo_cross),
                    },
                .ducking =
                    {
                        .enabled = ducking.value(QStringLiteral("enabled"), false).toBool(),
                        .amount_percent = static_cast<std::uint8_t>(ducking_amount),
                        .attack_millis = static_cast<std::uint16_t>(ducking_attack),
                        .release_millis = static_cast<std::uint16_t>(ducking_release),
                    },
            },
        .modulation =
            {
                .character =
                    static_cast<echo::audio::ModulationVfxCharacter>(*modulation_character),
                .enabled = modulation.value(QStringLiteral("enabled"), false).toBool(),
                .chorus =
                    {
                        .mix_percent = static_cast<std::uint8_t>(chorus_mix),
                        .rate_millihertz = static_cast<std::uint16_t>(chorus_rate),
                        .minimum_delay_microseconds = static_cast<std::uint16_t>(chorus_minimum),
                        .sweep_microseconds = static_cast<std::uint16_t>(chorus_sweep),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(chorus_phase),
                    },
                .flanger =
                    {
                        .mix_percent = static_cast<std::uint8_t>(flanger_mix),
                        .rate_millihertz = static_cast<std::uint16_t>(flanger_rate),
                        .minimum_delay_microseconds = static_cast<std::uint16_t>(flanger_minimum),
                        .sweep_microseconds = static_cast<std::uint16_t>(flanger_sweep),
                        .feedback_percent = static_cast<std::int8_t>(flanger_feedback),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(flanger_phase),
                    },
                .phaser =
                    {
                        .mix_percent = static_cast<std::uint8_t>(phaser_mix),
                        .rate_millihertz = static_cast<std::uint16_t>(phaser_rate),
                        .sweep_low_hertz = static_cast<std::uint16_t>(phaser_low),
                        .sweep_high_hertz = static_cast<std::uint16_t>(phaser_high),
                        .feedback_percent = static_cast<std::int8_t>(phaser_feedback),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(phaser_phase),
                    },
                .tremolo =
                    {
                        .rate_millihertz = static_cast<std::uint16_t>(tremolo_rate),
                        .depth_percent = static_cast<std::uint8_t>(tremolo_depth),
                        .stereo_phase_degrees = static_cast<std::uint16_t>(tremolo_phase),
                    },
            },
        .transform =
            {
                .character = static_cast<echo::audio::TransformVfxCharacter>(*transform_character),
                .enabled = transform.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(transform_mix),
                .amount_percent = static_cast<std::uint8_t>(transform_amount),
            },
        .digital_degrade =
            {
                .character = static_cast<echo::audio::DigitalDegradeVfxCharacter>(
                    *digital_degrade_character
                ),
                .enabled = digital_degrade.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(digital_mix),
                .bitcrusher = {.bit_depth = static_cast<std::uint8_t>(bit_depth)},
                .sample_rate_reduction =
                    {
                        .target_rate_hertz = static_cast<std::uint16_t>(target_rate),
                    },
            },
        .drive =
            {
                .character = static_cast<echo::audio::DriveVfxCharacter>(*drive_character),
                .enabled = drive.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(drive_mix),
                .drive_centibels = static_cast<std::uint16_t>(drive_amount),
                .tone_hertz = static_cast<std::uint16_t>(drive_tone),
                .output_gain_centibels = static_cast<std::int16_t>(drive_output),
            },
        .rotary =
            {
                .speed = static_cast<echo::audio::RotaryVfxSpeed>(*rotary_speed),
                .enabled = rotary.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(rotary_mix),
                .motion_percent = static_cast<std::uint8_t>(rotary_motion),
                .stereo_width_percent = static_cast<std::uint8_t>(rotary_width),
            },
        .freeze =
            {
                .enabled = freeze.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(freeze_mix),
                .capture_source_millis = static_cast<std::uint64_t>(freeze_anchor),
            },
        .granular =
            {
                .enabled = granular.value(QStringLiteral("enabled"), false).toBool(),
                .mix_percent = static_cast<std::uint8_t>(granular_mix),
                .grain_millis = static_cast<std::uint16_t>(grain_millis),
                .density_tenths_hertz = static_cast<std::uint16_t>(density),
                .lookback_millis = static_cast<std::uint16_t>(lookback),
                .scatter_millis = static_cast<std::uint16_t>(scatter),
                .pitch_cents = static_cast<std::int16_t>(pitch),
                .stereo_spread_percent = static_cast<std::uint8_t>(spread),
                .random_seed = static_cast<std::uint32_t>(seed),
            },
        .tape =
            {.enabled = tape.value(QStringLiteral("enabled"), false).toBool(),
             .mix_percent = static_cast<std::uint8_t>(tape_mix),
             .saturation_percent = static_cast<std::uint8_t>(tape_saturation),
             .wow_flutter_percent = static_cast<std::uint8_t>(tape_motion),
             .dropout_percent = static_cast<std::uint8_t>(tape_dropout)},
        .pitch =
            {.enabled = pitch_vfx.value(QStringLiteral("enabled"), false).toBool(),
             .mix_percent = static_cast<std::uint8_t>(pitch_mix),
             .pitch_semitones = static_cast<std::int8_t>(pitch_semitones),
             .harmony_enabled =
                 field(pitch_vfx, "harmonyEnabled", "harmony_enabled", false).toBool(),
             .harmony_semitones = static_cast<std::int8_t>(harmony_semitones),
             .harmony_mix_percent = static_cast<std::uint8_t>(harmony_mix),
             .formant_colour_semitones = static_cast<std::int8_t>(formant_colour)},
        .auto_wah = {
            .enabled = auto_wah.value(QStringLiteral("enabled"), false).toBool(),
            .mix_percent = static_cast<std::uint8_t>(wah_mix),
            .sensitivity_percent = static_cast<std::uint8_t>(wah_sensitivity),
            .minimum_frequency_hertz = static_cast<std::uint16_t>(wah_minimum),
            .maximum_frequency_hertz = static_cast<std::uint16_t>(wah_maximum),
            .resonance_tenths = static_cast<std::uint8_t>(wah_resonance)
        },
    };
}

std::optional<echo::audio::CreativeVfxAdjustment>
CreativeVfxProjection::fromJson(const QByteArray& encoded) {
    const QJsonDocument document = QJsonDocument::fromJson(encoded);
    return document.isObject() ? fromQml(document.object().toVariantMap()) : std::nullopt;
}

QVariantMap CreativeVfxProjection::toQml(const echo::audio::CreativeVfxAdjustment& adjustment) {
    QVariantMap scene =
        familyHeader(static_cast<int>(adjustment.scene.character), adjustment.scene.enabled);
    scene.insert(QStringLiteral("mixPercent"), adjustment.scene.mix_percent);
    scene.insert(QStringLiteral("intensityPercent"), adjustment.scene.intensity_percent);

    QVariantMap delay =
        familyHeader(static_cast<int>(adjustment.delay.character), adjustment.delay.enabled);
    delay.insert(
        QStringLiteral("slapback"),
        QVariantMap{
            {QStringLiteral("delayMillis"), adjustment.delay.slapback.delay_millis},
            {QStringLiteral("mixPercent"), adjustment.delay.slapback.mix_percent},
            {QStringLiteral("highCutHertz"), adjustment.delay.slapback.high_cut_hertz},
        }
    );
    delay.insert(
        QStringLiteral("echo"),
        QVariantMap{
            {QStringLiteral("delayMillis"), adjustment.delay.echo.delay_millis},
            {QStringLiteral("feedbackPercent"), adjustment.delay.echo.feedback_percent},
            {QStringLiteral("mixPercent"), adjustment.delay.echo.mix_percent},
            {QStringLiteral("highCutHertz"), adjustment.delay.echo.high_cut_hertz},
            {QStringLiteral("stereoCrossfeedPercent"),
             adjustment.delay.echo.stereo_crossfeed_percent},
        }
    );
    delay.insert(
        QStringLiteral("ducking"),
        QVariantMap{
            {QStringLiteral("enabled"), adjustment.delay.ducking.enabled},
            {QStringLiteral("amountPercent"), adjustment.delay.ducking.amount_percent},
            {QStringLiteral("attackMillis"), adjustment.delay.ducking.attack_millis},
            {QStringLiteral("releaseMillis"), adjustment.delay.ducking.release_millis},
        }
    );

    QVariantMap modulation = familyHeader(
        static_cast<int>(adjustment.modulation.character),
        adjustment.modulation.enabled
    );
    modulation.insert(
        QStringLiteral("chorus"),
        QVariantMap{
            {QStringLiteral("mixPercent"), adjustment.modulation.chorus.mix_percent},
            {QStringLiteral("rateMillihertz"), adjustment.modulation.chorus.rate_millihertz},
            {QStringLiteral("minimumDelayMicroseconds"),
             adjustment.modulation.chorus.minimum_delay_microseconds},
            {QStringLiteral("sweepMicroseconds"), adjustment.modulation.chorus.sweep_microseconds},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.chorus.stereo_phase_degrees},
        }
    );
    modulation.insert(
        QStringLiteral("flanger"),
        QVariantMap{
            {QStringLiteral("mixPercent"), adjustment.modulation.flanger.mix_percent},
            {QStringLiteral("rateMillihertz"), adjustment.modulation.flanger.rate_millihertz},
            {QStringLiteral("minimumDelayMicroseconds"),
             adjustment.modulation.flanger.minimum_delay_microseconds},
            {QStringLiteral("sweepMicroseconds"), adjustment.modulation.flanger.sweep_microseconds},
            {QStringLiteral("feedbackPercent"), adjustment.modulation.flanger.feedback_percent},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.flanger.stereo_phase_degrees},
        }
    );
    modulation.insert(
        QStringLiteral("phaser"),
        QVariantMap{
            {QStringLiteral("mixPercent"), adjustment.modulation.phaser.mix_percent},
            {QStringLiteral("rateMillihertz"), adjustment.modulation.phaser.rate_millihertz},
            {QStringLiteral("sweepLowHertz"), adjustment.modulation.phaser.sweep_low_hertz},
            {QStringLiteral("sweepHighHertz"), adjustment.modulation.phaser.sweep_high_hertz},
            {QStringLiteral("feedbackPercent"), adjustment.modulation.phaser.feedback_percent},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.phaser.stereo_phase_degrees},
        }
    );
    modulation.insert(
        QStringLiteral("tremolo"),
        QVariantMap{
            {QStringLiteral("rateMillihertz"), adjustment.modulation.tremolo.rate_millihertz},
            {QStringLiteral("depthPercent"), adjustment.modulation.tremolo.depth_percent},
            {QStringLiteral("stereoPhaseDegrees"),
             adjustment.modulation.tremolo.stereo_phase_degrees},
        }
    );

    QVariantMap transform = familyHeader(
        static_cast<int>(adjustment.transform.character),
        adjustment.transform.enabled
    );
    transform.insert(QStringLiteral("mixPercent"), adjustment.transform.mix_percent);
    transform.insert(QStringLiteral("amountPercent"), adjustment.transform.amount_percent);
    QVariantMap digital_degrade = familyHeader(
        static_cast<int>(adjustment.digital_degrade.character),
        adjustment.digital_degrade.enabled
    );
    digital_degrade.insert(QStringLiteral("mixPercent"), adjustment.digital_degrade.mix_percent);
    digital_degrade.insert(
        QStringLiteral("bitcrusher"),
        QVariantMap{{QStringLiteral("bitDepth"), adjustment.digital_degrade.bitcrusher.bit_depth}}
    );
    QVariantMap drive =
        familyHeader(static_cast<int>(adjustment.drive.character), adjustment.drive.enabled);
    drive.insert(QStringLiteral("mixPercent"), adjustment.drive.mix_percent);
    drive.insert(QStringLiteral("driveCentibels"), adjustment.drive.drive_centibels);
    drive.insert(QStringLiteral("toneHertz"), adjustment.drive.tone_hertz);
    drive.insert(QStringLiteral("outputGainCentibels"), adjustment.drive.output_gain_centibels);
    QVariantMap rotary{
        {QStringLiteral("speed"), static_cast<int>(adjustment.rotary.speed)},
        {QStringLiteral("enabled"), adjustment.rotary.enabled},
        {QStringLiteral("mixPercent"), adjustment.rotary.mix_percent},
        {QStringLiteral("motionPercent"), adjustment.rotary.motion_percent},
        {QStringLiteral("stereoWidthPercent"), adjustment.rotary.stereo_width_percent},
    };
    QVariantMap freeze{
        {QStringLiteral("enabled"), adjustment.freeze.enabled},
        {QStringLiteral("mixPercent"), adjustment.freeze.mix_percent},
        {QStringLiteral("captureSourceMillis"),
         QVariant::fromValue<qulonglong>(adjustment.freeze.capture_source_millis)},
    };
    QVariantMap granular{
        {QStringLiteral("enabled"), adjustment.granular.enabled},
        {QStringLiteral("mixPercent"), adjustment.granular.mix_percent},
        {QStringLiteral("grainMillis"), adjustment.granular.grain_millis},
        {QStringLiteral("densityTenthsHertz"), adjustment.granular.density_tenths_hertz},
        {QStringLiteral("lookbackMillis"), adjustment.granular.lookback_millis},
        {QStringLiteral("scatterMillis"), adjustment.granular.scatter_millis},
        {QStringLiteral("pitchCents"), adjustment.granular.pitch_cents},
        {QStringLiteral("stereoSpreadPercent"), adjustment.granular.stereo_spread_percent},
        {QStringLiteral("randomSeed"),
         QVariant::fromValue<qulonglong>(adjustment.granular.random_seed)},
    };
    QVariantMap tape{
        {QStringLiteral("enabled"), adjustment.tape.enabled},
        {QStringLiteral("mixPercent"), adjustment.tape.mix_percent},
        {QStringLiteral("saturationPercent"), adjustment.tape.saturation_percent},
        {QStringLiteral("wowFlutterPercent"), adjustment.tape.wow_flutter_percent},
        {QStringLiteral("dropoutPercent"), adjustment.tape.dropout_percent},
    };
    QVariantMap pitch{
        {QStringLiteral("enabled"), adjustment.pitch.enabled},
        {QStringLiteral("mixPercent"), adjustment.pitch.mix_percent},
        {QStringLiteral("pitchSemitones"), adjustment.pitch.pitch_semitones},
        {QStringLiteral("harmonyEnabled"), adjustment.pitch.harmony_enabled},
        {QStringLiteral("harmonySemitones"), adjustment.pitch.harmony_semitones},
        {QStringLiteral("harmonyMixPercent"), adjustment.pitch.harmony_mix_percent},
        {QStringLiteral("formantColourSemitones"), adjustment.pitch.formant_colour_semitones},
    };
    QVariantMap auto_wah{
        {QStringLiteral("enabled"), adjustment.auto_wah.enabled},
        {QStringLiteral("mixPercent"), adjustment.auto_wah.mix_percent},
        {QStringLiteral("sensitivityPercent"), adjustment.auto_wah.sensitivity_percent},
        {QStringLiteral("minimumFrequencyHertz"), adjustment.auto_wah.minimum_frequency_hertz},
        {QStringLiteral("maximumFrequencyHertz"), adjustment.auto_wah.maximum_frequency_hertz},
        {QStringLiteral("resonanceTenths"), adjustment.auto_wah.resonance_tenths},
    };
    digital_degrade.insert(
        QStringLiteral("sampleRateReduction"),
        QVariantMap{
            {QStringLiteral("targetRateHertz"),
             adjustment.digital_degrade.sample_rate_reduction.target_rate_hertz}
        }
    );
    return {
        {QStringLiteral("scene"), scene},
        {QStringLiteral("delay"), delay},
        {QStringLiteral("modulation"), modulation},
        {QStringLiteral("transform"), transform},
        {QStringLiteral("digitalDegrade"), digital_degrade},
        {QStringLiteral("drive"), drive},
        {QStringLiteral("rotary"), rotary},
        {QStringLiteral("freeze"), freeze},
        {QStringLiteral("granular"), granular},
        {QStringLiteral("tape"), tape},
        {QStringLiteral("pitch"), pitch},
        {QStringLiteral("autoWah"), auto_wah},
    };
}

QByteArray CreativeVfxProjection::toJson(const echo::audio::CreativeVfxAdjustment& adjustment) {
    const QVariantMap qml = toQml(adjustment);
    auto enum_name = [](int value, const QStringList& names) { return names.at(value); };
    QVariantMap scene = nested(qml, "scene");
    scene[QStringLiteral("character")] = enum_name(
        scene.value(QStringLiteral("character")).toInt(),
        {"telephone", "radio", "intercom", "behind_wall", "underwater"}
    );
    QVariantMap delay = nested(qml, "delay");
    delay[QStringLiteral("character")] =
        enum_name(delay.value(QStringLiteral("character")).toInt(), {"slapback", "echo"});
    QVariantMap modulation = nested(qml, "modulation");
    modulation[QStringLiteral("character")] = enum_name(
        modulation.value(QStringLiteral("character")).toInt(),
        {"chorus", "flanger", "phaser", "tremolo"}
    );
    QVariantMap transform = nested(qml, "transform");
    transform[QStringLiteral("character")] = enum_name(
        transform.value(QStringLiteral("character")).toInt(),
        {"robot", "monster", "tiny", "giant", "ghost"}
    );
    QVariantMap digital_degrade = nested(qml, "digitalDegrade");
    digital_degrade[QStringLiteral("character")] = enum_name(
        digital_degrade.value(QStringLiteral("character")).toInt(),
        {"bitcrusher", "sample_rate_reduction", "lo_fi"}
    );
    QVariantMap drive = nested(qml, "drive");
    drive[QStringLiteral("character")] = enum_name(
        drive.value(QStringLiteral("character")).toInt(),
        {"soft_clip", "overdrive", "fuzz"}
    );
    QVariantMap rotary = nested(qml, "rotary");
    rotary[QStringLiteral("speed")] =
        enum_name(rotary.value(QStringLiteral("speed")).toInt(), {"slow", "fast", "brake"});
    auto snake = [](QVariantMap map, const QString& camel, const QString& snake_name) {
        map.insert(snake_name, map.take(camel));
        return map;
    };
    scene = snake(scene, "mixPercent", "mix_percent");
    scene = snake(scene, "intensityPercent", "intensity_percent");
    auto delay_params = [&](QVariantMap map) {
        map = snake(map, "delayMillis", "delay_millis");
        map = snake(map, "mixPercent", "mix_percent");
        map = snake(map, "highCutHertz", "high_cut_hertz");
        return map;
    };
    QVariantMap slapback = delay_params(nested(delay, "slapback"));
    QVariantMap echo = delay_params(nested(delay, "echo"));
    echo = snake(echo, "feedbackPercent", "feedback_percent");
    echo = snake(echo, "stereoCrossfeedPercent", "stereo_crossfeed_percent");
    QVariantMap ducking = nested(delay, "ducking");
    ducking = snake(ducking, "amountPercent", "amount_percent");
    ducking = snake(ducking, "attackMillis", "attack_millis");
    ducking = snake(ducking, "releaseMillis", "release_millis");
    delay[QStringLiteral("slapback")] = slapback;
    delay[QStringLiteral("echo")] = echo;
    delay[QStringLiteral("ducking")] = ducking;
    auto common_modulation = [&](QVariantMap map) {
        map = snake(map, "rateMillihertz", "rate_millihertz");
        map = snake(map, "stereoPhaseDegrees", "stereo_phase_degrees");
        return map;
    };
    QVariantMap chorus = common_modulation(nested(modulation, "chorus"));
    chorus = snake(chorus, "mixPercent", "mix_percent");
    chorus = snake(chorus, "minimumDelayMicroseconds", "minimum_delay_microseconds");
    chorus = snake(chorus, "sweepMicroseconds", "sweep_microseconds");
    QVariantMap flanger = common_modulation(nested(modulation, "flanger"));
    flanger = snake(flanger, "mixPercent", "mix_percent");
    flanger = snake(flanger, "minimumDelayMicroseconds", "minimum_delay_microseconds");
    flanger = snake(flanger, "sweepMicroseconds", "sweep_microseconds");
    flanger = snake(flanger, "feedbackPercent", "feedback_percent");
    QVariantMap phaser = common_modulation(nested(modulation, "phaser"));
    phaser = snake(phaser, "mixPercent", "mix_percent");
    phaser = snake(phaser, "sweepLowHertz", "sweep_low_hertz");
    phaser = snake(phaser, "sweepHighHertz", "sweep_high_hertz");
    phaser = snake(phaser, "feedbackPercent", "feedback_percent");
    QVariantMap tremolo = common_modulation(nested(modulation, "tremolo"));
    tremolo = snake(tremolo, "depthPercent", "depth_percent");
    modulation[QStringLiteral("chorus")] = chorus;
    modulation[QStringLiteral("flanger")] = flanger;
    modulation[QStringLiteral("phaser")] = phaser;
    modulation[QStringLiteral("tremolo")] = tremolo;
    transform = snake(transform, "mixPercent", "mix_percent");
    transform = snake(transform, "amountPercent", "amount_percent");
    digital_degrade = snake(digital_degrade, "mixPercent", "mix_percent");
    QVariantMap digital_bitcrusher = nested(digital_degrade, "bitcrusher");
    digital_bitcrusher = snake(digital_bitcrusher, "bitDepth", "bit_depth");
    QVariantMap digital_rate = nested(digital_degrade, "sampleRateReduction");
    digital_rate = snake(digital_rate, "targetRateHertz", "target_rate_hertz");
    digital_degrade[QStringLiteral("bitcrusher")] = digital_bitcrusher;
    digital_degrade.remove(QStringLiteral("sampleRateReduction"));
    digital_degrade[QStringLiteral("sample_rate_reduction")] = digital_rate;
    drive = snake(drive, "mixPercent", "mix_percent");
    drive = snake(drive, "driveCentibels", "drive_centibels");
    drive = snake(drive, "toneHertz", "tone_hertz");
    drive = snake(drive, "outputGainCentibels", "output_gain_centibels");
    rotary = snake(rotary, "mixPercent", "mix_percent");
    rotary = snake(rotary, "motionPercent", "motion_percent");
    rotary = snake(rotary, "stereoWidthPercent", "stereo_width_percent");
    QVariantMap freeze = nested(qml, "freeze");
    freeze = snake(freeze, "mixPercent", "mix_percent");
    freeze = snake(freeze, "captureSourceMillis", "capture_source_millis");
    QVariantMap granular = nested(qml, "granular");
    granular = snake(granular, "mixPercent", "mix_percent");
    granular = snake(granular, "grainMillis", "grain_millis");
    granular = snake(granular, "densityTenthsHertz", "density_tenths_hertz");
    granular = snake(granular, "lookbackMillis", "lookback_millis");
    granular = snake(granular, "scatterMillis", "scatter_millis");
    granular = snake(granular, "pitchCents", "pitch_cents");
    granular = snake(granular, "stereoSpreadPercent", "stereo_spread_percent");
    granular = snake(granular, "randomSeed", "random_seed");
    QVariantMap tape = nested(qml, "tape");
    tape = snake(tape, "mixPercent", "mix_percent");
    tape = snake(tape, "saturationPercent", "saturation_percent");
    tape = snake(tape, "wowFlutterPercent", "wow_flutter_percent");
    tape = snake(tape, "dropoutPercent", "dropout_percent");
    QVariantMap pitch = nested(qml, "pitch");
    pitch = snake(pitch, "mixPercent", "mix_percent");
    pitch = snake(pitch, "pitchSemitones", "pitch_semitones");
    pitch = snake(pitch, "harmonyEnabled", "harmony_enabled");
    pitch = snake(pitch, "harmonySemitones", "harmony_semitones");
    pitch = snake(pitch, "harmonyMixPercent", "harmony_mix_percent");
    pitch = snake(pitch, "formantColourSemitones", "formant_colour_semitones");
    QVariantMap auto_wah = nested(qml, "autoWah");
    auto_wah = snake(auto_wah, "mixPercent", "mix_percent");
    auto_wah = snake(auto_wah, "sensitivityPercent", "sensitivity_percent");
    auto_wah = snake(auto_wah, "minimumFrequencyHertz", "minimum_frequency_hertz");
    auto_wah = snake(auto_wah, "maximumFrequencyHertz", "maximum_frequency_hertz");
    auto_wah = snake(auto_wah, "resonanceTenths", "resonance_tenths");
    return QJsonDocument::fromVariant(
               QVariantMap{
                   {QStringLiteral("scene"), scene},
                   {QStringLiteral("delay"), delay},
                   {QStringLiteral("modulation"), modulation},
                   {QStringLiteral("transform"), transform},
                   {QStringLiteral("digital_degrade"), digital_degrade},
                   {QStringLiteral("drive"), drive},
                   {QStringLiteral("rotary"), rotary},
                   {QStringLiteral("freeze"), freeze},
                   {QStringLiteral("granular"), granular},
                   {QStringLiteral("tape"), tape},
                   {QStringLiteral("pitch"), pitch},
                   {QStringLiteral("auto_wah"), auto_wah},
               }
    )
        .toJson(QJsonDocument::Compact);
}
