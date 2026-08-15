#include "creative_vfx_projection.hpp"

#include <QJsonDocument>
#include <QJsonObject>

#include <cassert>

int main() {
    echo::audio::CreativeVfxAdjustment authored;
    authored.scene = {
        .character = echo::audio::SceneVfxCharacter::Underwater,
        .enabled = true,
        .mix_percent = 76,
        .intensity_percent = 63,
    };
    authored.delay.character = echo::audio::DelayVfxCharacter::Echo;
    authored.delay.enabled = true;
    authored.delay.echo = {
        .delay_millis = 420,
        .feedback_percent = 44,
        .mix_percent = 31,
        .high_cut_hertz = 5800,
        .stereo_crossfeed_percent = 82,
    };
    authored.delay.ducking = {
        .enabled = true,
        .amount_percent = 72,
        .attack_millis = 15,
        .release_millis = 380,
    };
    authored.modulation.character = echo::audio::ModulationVfxCharacter::Phaser;
    authored.modulation.enabled = true;
    authored.modulation.phaser = {
        .mix_percent = 47,
        .rate_millihertz = 620,
        .sweep_low_hertz = 240,
        .sweep_high_hertz = 3100,
        .feedback_percent = -18,
        .stereo_phase_degrees = 120,
    };
    authored.transform = {
        .character = echo::audio::TransformVfxCharacter::Ghost,
        .enabled = true,
        .mix_percent = 84,
        .amount_percent = 71,
    };
    authored.digital_degrade = {
        .character = echo::audio::DigitalDegradeVfxCharacter::LoFi,
        .enabled = true,
        .mix_percent = 73,
        .bitcrusher = {.bit_depth = 7},
        .sample_rate_reduction = {.target_rate_hertz = 11025},
    };
    authored.drive = {
        .character = echo::audio::DriveVfxCharacter::Fuzz,
        .enabled = true,
        .mix_percent = 68,
        .drive_centibels = 2450,
        .tone_hertz = 6200,
        .output_gain_centibels = -475,
    };
    authored.rotary = {
        .speed = echo::audio::RotaryVfxSpeed::Fast,
        .enabled = true,
        .mix_percent = 62,
        .motion_percent = 74,
        .stereo_width_percent = 91,
    };
    authored.freeze = {
        .enabled = true,
        .mix_percent = 66,
        .capture_source_millis = 2400,
    };
    authored.granular = {
        .enabled = true,
        .mix_percent = 58,
        .grain_millis = 95,
        .density_tenths_hertz = 175,
        .lookback_millis = 320,
        .scatter_millis = 180,
        .pitch_cents = -350,
        .stereo_spread_percent = 73,
        .random_seed = 0x12345678U,
    };
    authored.tape = {
        .enabled = true,
        .mix_percent = 67,
        .saturation_percent = 48,
        .wow_flutter_percent = 36,
        .dropout_percent = 9,
    };
    authored.pitch = {
        .enabled = true,
        .mix_percent = 81,
        .pitch_semitones = -3,
        .harmony_enabled = true,
        .harmony_semitones = 7,
        .harmony_mix_percent = 42,
        .formant_colour_semitones = 2,
    };
    authored.auto_wah = {
        .enabled = true,
        .mix_percent = 73,
        .sensitivity_percent = 61,
        .minimum_frequency_hertz = 310,
        .maximum_frequency_hertz = 3400,
        .resonance_tenths = 22,
    };
    authored.stereo = {
        .enabled = true,
        .mix_percent = 84,
        .width_percent = 146,
        .pan_percent = -18,
    };
    authored.beat_repeat = {
        .enabled = true,
        .mix_percent = 72,
        .slice_millis = 180,
        .repeat_count = 4,
        .reverse = true,
    };

    const QVariantMap qml = CreativeVfxProjection::toQml(authored);
    const auto from_qml = CreativeVfxProjection::fromQml(qml);
    assert(from_qml.has_value());
    assert(from_qml->scene.character == echo::audio::SceneVfxCharacter::Underwater);
    assert(from_qml->delay.echo.feedback_percent == 44);
    assert(from_qml->delay.ducking.amount_percent == 72);
    assert(from_qml->delay.ducking.release_millis == 380);
    assert(from_qml->modulation.phaser.feedback_percent == -18);
    assert(from_qml->transform.character == echo::audio::TransformVfxCharacter::Ghost);
    assert(from_qml->digital_degrade.character == echo::audio::DigitalDegradeVfxCharacter::LoFi);
    assert(from_qml->drive.character == echo::audio::DriveVfxCharacter::Fuzz);
    assert(from_qml->drive.drive_centibels == 2450);
    assert(from_qml->rotary.speed == echo::audio::RotaryVfxSpeed::Fast);
    assert(from_qml->rotary.stereo_width_percent == 91);
    assert(from_qml->freeze.capture_source_millis == 2400);
    assert(from_qml->granular.pitch_cents == -350);
    assert(from_qml->granular.random_seed == 0x12345678U);
    assert(from_qml->tape.wow_flutter_percent == 36);
    assert(from_qml->pitch.harmony_enabled);
    assert(from_qml->pitch.formant_colour_semitones == 2);
    assert(from_qml->auto_wah.maximum_frequency_hertz == 3400);
    assert(from_qml->stereo.width_percent == 146);
    assert(from_qml->beat_repeat.reverse);

    const QByteArray encoded = CreativeVfxProjection::toJson(authored);
    const QJsonObject json = QJsonDocument::fromJson(encoded).object();
    assert(
        json.value(QStringLiteral("scene")).toObject().value(QStringLiteral("character"))
        == QStringLiteral("underwater")
    );
    assert(json.value(QStringLiteral("scene")).toObject().contains(QStringLiteral("mix_percent")));
    assert(!json.value(QStringLiteral("scene")).toObject().contains(QStringLiteral("mixPercent")));
    assert(
        json.value(QStringLiteral("drive")).toObject().value(QStringLiteral("character"))
        == QStringLiteral("fuzz")
    );
    assert(
        json.value(QStringLiteral("rotary")).toObject().value(QStringLiteral("speed"))
        == QStringLiteral("fast")
    );
    assert(
        json.value(QStringLiteral("freeze"))
            .toObject()
            .value(QStringLiteral("capture_source_millis"))
        == 2400
    );
    assert(
        json.value(QStringLiteral("granular")).toObject().value(QStringLiteral("pitch_cents"))
        == -350
    );
    assert(
        json.value(QStringLiteral("tape")).toObject().value(QStringLiteral("wow_flutter_percent"))
        == 36
    );
    assert(json.value(QStringLiteral("pitch"))
               .toObject()
               .value(QStringLiteral("harmony_enabled"))
               .toBool());
    assert(
        json.value(QStringLiteral("auto_wah"))
            .toObject()
            .value(QStringLiteral("maximum_frequency_hertz"))
        == 3400
    );
    assert(json.value(QStringLiteral("stereo")).toObject().value(QStringLiteral("pan_percent")) == -18);
    assert(json.value(QStringLiteral("beat_repeat")).toObject().value(QStringLiteral("reverse")).toBool());
    const auto from_json = CreativeVfxProjection::fromJson(encoded);
    assert(from_json.has_value());
    assert(CreativeVfxProjection::toJson(*from_json) == encoded);

    QVariantMap invalid = qml;
    QVariantMap scene = invalid.value(QStringLiteral("scene")).toMap();
    scene.insert(QStringLiteral("mixPercent"), 101);
    invalid.insert(QStringLiteral("scene"), scene);
    assert(!CreativeVfxProjection::fromQml(invalid).has_value());
    invalid = qml;
    QVariantMap drive = invalid.value(QStringLiteral("drive")).toMap();
    drive.insert(QStringLiteral("toneHertz"), 499);
    invalid.insert(QStringLiteral("drive"), drive);
    assert(!CreativeVfxProjection::fromQml(invalid).has_value());
    assert(!CreativeVfxProjection::fromJson(QByteArrayLiteral("[]")).has_value());

    const auto defaults = CreativeVfxProjection::fromQml({});
    assert(defaults.has_value());
    assert(!defaults->scene.enabled);
    assert(!defaults->delay.enabled);
    assert(!defaults->delay.ducking.enabled);
    assert(defaults->delay.ducking.amount_percent == 65);
    assert(!defaults->modulation.enabled);
    assert(!defaults->transform.enabled);
    assert(!defaults->digital_degrade.enabled);
    assert(!defaults->drive.enabled);
    assert(!defaults->rotary.enabled);
    assert(!defaults->freeze.enabled);
    assert(defaults->freeze.capture_source_millis == 100);
    assert(!defaults->granular.enabled);
    assert(!defaults->tape.enabled);
    assert(!defaults->pitch.enabled);
    assert(!defaults->auto_wah.enabled);
    assert(!defaults->stereo.enabled);
    assert(!defaults->beat_repeat.enabled);

    invalid = qml;
    QVariantMap delay = invalid.value(QStringLiteral("delay")).toMap();
    QVariantMap ducking = delay.value(QStringLiteral("ducking")).toMap();
    ducking.insert(QStringLiteral("attackMillis"), 0);
    delay.insert(QStringLiteral("ducking"), ducking);
    invalid.insert(QStringLiteral("delay"), delay);
    assert(!CreativeVfxProjection::fromQml(invalid).has_value());

    invalid = qml;
    QVariantMap freeze = invalid.value(QStringLiteral("freeze")).toMap();
    freeze.insert(QStringLiteral("captureSourceMillis"), 85);
    invalid.insert(QStringLiteral("freeze"), freeze);
    assert(!CreativeVfxProjection::fromQml(invalid).has_value());

    invalid = qml;
    QVariantMap granular = invalid.value(QStringLiteral("granular")).toMap();
    granular.insert(QStringLiteral("lookbackMillis"), 1500);
    granular.insert(QStringLiteral("scatterMillis"), 750);
    granular.insert(QStringLiteral("grainMillis"), 250);
    granular.insert(QStringLiteral("pitchCents"), 1200);
    invalid.insert(QStringLiteral("granular"), granular);
    assert(!CreativeVfxProjection::fromQml(invalid).has_value());
}
