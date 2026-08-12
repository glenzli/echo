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

    const QVariantMap qml = CreativeVfxProjection::toQml(authored);
    const auto from_qml = CreativeVfxProjection::fromQml(qml);
    assert(from_qml.has_value());
    assert(from_qml->scene.character == echo::audio::SceneVfxCharacter::Underwater);
    assert(from_qml->delay.echo.feedback_percent == 44);
    assert(from_qml->modulation.phaser.feedback_percent == -18);
    assert(from_qml->transform.character == echo::audio::TransformVfxCharacter::Ghost);
    assert(from_qml->digital_degrade.character == echo::audio::DigitalDegradeVfxCharacter::LoFi);

    const QByteArray encoded = CreativeVfxProjection::toJson(authored);
    const QJsonObject json = QJsonDocument::fromJson(encoded).object();
    assert(json.value(QStringLiteral("scene")).toObject().value(QStringLiteral("character"))
           == QStringLiteral("underwater"));
    assert(json.value(QStringLiteral("scene")).toObject().contains(QStringLiteral("mix_percent")));
    assert(!json.value(QStringLiteral("scene")).toObject().contains(QStringLiteral("mixPercent")));
    const auto from_json = CreativeVfxProjection::fromJson(encoded);
    assert(from_json.has_value());
    assert(CreativeVfxProjection::toJson(*from_json) == encoded);

    QVariantMap invalid = qml;
    QVariantMap scene = invalid.value(QStringLiteral("scene")).toMap();
    scene.insert(QStringLiteral("mixPercent"), 101);
    invalid.insert(QStringLiteral("scene"), scene);
    assert(!CreativeVfxProjection::fromQml(invalid).has_value());
    assert(!CreativeVfxProjection::fromJson(QByteArrayLiteral("[]")).has_value());

    const auto defaults = CreativeVfxProjection::fromQml({});
    assert(defaults.has_value());
    assert(!defaults->scene.enabled);
    assert(!defaults->delay.enabled);
    assert(!defaults->modulation.enabled);
    assert(!defaults->transform.enabled);
    assert(!defaults->digital_degrade.enabled);
}
