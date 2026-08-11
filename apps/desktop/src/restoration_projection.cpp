#include "restoration_projection.hpp"

std::optional<echo::audio::RestorationAdjustment>
RestorationProjection::fromQml(const QVariantMap& value) {
    const bool enabled = value.value(QStringLiteral("enabled"), true).toBool();
    const bool de_plosive_enabled = value.value(QStringLiteral("dePlosiveEnabled"), false).toBool();
    const int de_plosive_frequency =
        value.value(QStringLiteral("dePlosiveFrequencyHertz"), 140).toInt();
    const int de_plosive_sensitivity =
        value.value(QStringLiteral("dePlosiveSensitivityPercent"), 50).toInt();
    const int de_plosive_reduction =
        value.value(QStringLiteral("dePlosiveReductionCentibels"), 1200).toInt();
    const int de_plosive_release =
        value.value(QStringLiteral("dePlosiveReleaseMillis"), 160).toInt();
    const bool noise_enabled = value.value(QStringLiteral("noiseEnabled"), false).toBool();
    const int noise_reduction = value.value(QStringLiteral("noiseReductionCentibels"), 900).toInt();
    const int noise_sensitivity =
        value.value(QStringLiteral("noiseSensitivityPercent"), 50).toInt();
    const int noise_smoothing = value.value(QStringLiteral("noiseSmoothingMillis"), 240).toInt();
    const bool de_esser_enabled = value.value(QStringLiteral("deEsserEnabled"), false).toBool();
    const int de_esser_frequency =
        value.value(QStringLiteral("deEsserFrequencyHertz"), 6500).toInt();
    const int de_esser_threshold =
        value.value(QStringLiteral("deEsserThresholdCentibels"), -2400).toInt();
    const int de_esser_reduction =
        value.value(QStringLiteral("deEsserReductionCentibels"), 600).toInt();
    if (de_plosive_frequency < 80 || de_plosive_frequency > 240 || de_plosive_sensitivity < 0
        || de_plosive_sensitivity > 100 || de_plosive_reduction < 0 || de_plosive_reduction > 1800
        || de_plosive_release < 40 || de_plosive_release > 500 || noise_reduction < 0
        || noise_reduction > 2400 || noise_sensitivity < 0 || noise_sensitivity > 100
        || noise_smoothing < 20 || noise_smoothing > 1000 || de_esser_frequency < 3000
        || de_esser_frequency > 12000 || de_esser_threshold < -6000 || de_esser_threshold > 0
        || de_esser_reduction < 0 || de_esser_reduction > 1800) {
        return std::nullopt;
    }
    return echo::audio::RestorationAdjustment{
        .enabled = enabled,
        .de_plosive =
            {
                .enabled = de_plosive_enabled,
                .frequency_hertz = static_cast<std::uint16_t>(de_plosive_frequency),
                .sensitivity_percent = static_cast<std::uint8_t>(de_plosive_sensitivity),
                .reduction_centibels = static_cast<std::uint16_t>(de_plosive_reduction),
                .release_millis = static_cast<std::uint16_t>(de_plosive_release),
            },
        .noise_reduction =
            {
                .enabled = noise_enabled,
                .reduction_centibels = static_cast<std::uint16_t>(noise_reduction),
                .sensitivity_percent = static_cast<std::uint8_t>(noise_sensitivity),
                .smoothing_millis = static_cast<std::uint16_t>(noise_smoothing),
            },
        .de_esser = {
            .enabled = de_esser_enabled,
            .frequency_hertz = static_cast<std::uint16_t>(de_esser_frequency),
            .threshold_centibels = static_cast<std::int16_t>(de_esser_threshold),
            .reduction_centibels = static_cast<std::uint16_t>(de_esser_reduction),
        },
    };
}
