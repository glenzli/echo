#include "reverb_projection.hpp"

std::optional<echo::audio::ReverbAdjustment> ReverbProjection::fromQml(const QVariantMap& value) {
    const int character = value.value(QStringLiteral("character"), 0).toInt();
    const int mix = value.value(QStringLiteral("mixPercent"), 18).toInt();
    const int pre_delay = value.value(QStringLiteral("preDelayMillis"), 20).toInt();
    const int decay = value.value(QStringLiteral("decayMillis"), 1800).toInt();
    const int size = value.value(QStringLiteral("sizePercent"), 55).toInt();
    const int damping = value.value(QStringLiteral("dampingPercent"), 45).toInt();
    const int low_cut = value.value(QStringLiteral("lowCutHertz"), 120).toInt();
    const int high_cut = value.value(QStringLiteral("highCutHertz"), 10000).toInt();
    const QVariantMap ducking = value.value(QStringLiteral("ducking")).toMap();
    const int ducking_amount = ducking.value(QStringLiteral("amountPercent"), 65).toInt();
    const int ducking_attack = ducking.value(QStringLiteral("attackMillis"), 10).toInt();
    const int ducking_release = ducking.value(QStringLiteral("releaseMillis"), 250).toInt();
    if (character < 0 || character > 3 || mix < 0 || mix > 100 || pre_delay < 0 || pre_delay > 200
        || decay < 100 || decay > 12000 || size < 10 || size > 100 || damping < 0 || damping > 100
        || low_cut < 20 || low_cut > 1000 || high_cut < 1000 || high_cut > 20000
        || low_cut >= high_cut || ducking_amount < 0 || ducking_amount > 100 || ducking_attack < 1
        || ducking_attack > 200 || ducking_release < 20 || ducking_release > 2000) {
        return std::nullopt;
    }
    return echo::audio::ReverbAdjustment{
        .character = static_cast<echo::audio::ReverbCharacter>(character),
        .enabled = value.value(QStringLiteral("enabled"), false).toBool(),
        .mix_percent = static_cast<std::uint8_t>(mix),
        .pre_delay_millis = static_cast<std::uint16_t>(pre_delay),
        .decay_millis = static_cast<std::uint16_t>(decay),
        .size_percent = static_cast<std::uint8_t>(size),
        .damping_percent = static_cast<std::uint8_t>(damping),
        .low_cut_hertz = static_cast<std::uint16_t>(low_cut),
        .high_cut_hertz = static_cast<std::uint16_t>(high_cut),
        .ducking = {
            .enabled = ducking.value(QStringLiteral("enabled"), false).toBool(),
            .amount_percent = static_cast<std::uint8_t>(ducking_amount),
            .attack_millis = static_cast<std::uint16_t>(ducking_attack),
            .release_millis = static_cast<std::uint16_t>(ducking_release),
        },
    };
}
