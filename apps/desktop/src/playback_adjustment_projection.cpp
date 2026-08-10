#include "playback_adjustment_projection.hpp"

#include "parametric_equalizer_projection.hpp"
#include "restoration_projection.hpp"
#include "reverb_projection.hpp"

std::optional<echo::audio::PlaybackAdjustment>
PlaybackAdjustmentProjection::fromAssetMap(const QVariantMap& asset) {
    const QVariantMap restoration{
        {QStringLiteral("noiseEnabled"), asset.value(QStringLiteral("noiseReductionEnabled"))},
        {QStringLiteral("noiseReductionCentibels"),
         asset.value(QStringLiteral("noiseReductionCentibels"))},
        {QStringLiteral("noiseSensitivityPercent"),
         asset.value(QStringLiteral("noiseReductionSensitivityPercent"))},
        {QStringLiteral("noiseSmoothingMillis"),
         asset.value(QStringLiteral("noiseReductionSmoothingMillis"))},
        {QStringLiteral("deEsserEnabled"), asset.value(QStringLiteral("deEsserEnabled"))},
        {QStringLiteral("deEsserFrequencyHertz"),
         asset.value(QStringLiteral("deEsserFrequencyHertz"))},
        {QStringLiteral("deEsserThresholdCentibels"),
         asset.value(QStringLiteral("deEsserThresholdCentibels"))},
        {QStringLiteral("deEsserReductionCentibels"),
         asset.value(QStringLiteral("deEsserReductionCentibels"))},
    };
    const QVariantMap reverb{
        {QStringLiteral("enabled"), asset.value(QStringLiteral("reverbEnabled"))},
        {QStringLiteral("mixPercent"), asset.value(QStringLiteral("reverbMixPercent"))},
        {QStringLiteral("preDelayMillis"), asset.value(QStringLiteral("reverbPreDelayMillis"))},
        {QStringLiteral("decayMillis"), asset.value(QStringLiteral("reverbDecayMillis"))},
        {QStringLiteral("sizePercent"), asset.value(QStringLiteral("reverbSizePercent"))},
        {QStringLiteral("dampingPercent"), asset.value(QStringLiteral("reverbDampingPercent"))},
        {QStringLiteral("lowCutHertz"), asset.value(QStringLiteral("reverbLowCutHertz"))},
        {QStringLiteral("highCutHertz"), asset.value(QStringLiteral("reverbHighCutHertz"))},
    };
    return fromQml(
        asset.value(QStringLiteral("trimStartMillis")).toLongLong(),
        asset.value(QStringLiteral("trimEndMillis")).toLongLong(),
        asset.value(QStringLiteral("fadeInMillis")).toLongLong(),
        asset.value(QStringLiteral("fadeOutMillis")).toLongLong(),
        asset.value(QStringLiteral("fadeInCurve")).toInt(),
        asset.value(QStringLiteral("fadeOutCurve")).toInt(),
        asset.value(QStringLiteral("gainCentibels")).toInt(),
        asset.value(QStringLiteral("lowCutHertz")).toInt(),
        restoration,
        asset.value(QStringLiteral("equalizerBands")).toList(),
        asset.value(QStringLiteral("compressorEnabled")).toBool(),
        asset.value(QStringLiteral("compressorThresholdCentibels")).toInt(),
        asset.value(QStringLiteral("compressorRatioTenths")).toInt(),
        asset.value(QStringLiteral("compressorAttackMillis")).toInt(),
        asset.value(QStringLiteral("compressorReleaseMillis")).toInt(),
        asset.value(QStringLiteral("compressorMakeupCentibels")).toInt(),
        reverb,
        asset.value(QStringLiteral("limiterEnabled")).toBool(),
        asset.value(QStringLiteral("limiterCeilingCentibels")).toInt(),
        asset.value(QStringLiteral("limiterReleaseMillis")).toInt()
    );
}

std::optional<echo::audio::PlaybackAdjustment> PlaybackAdjustmentProjection::fromQml(
    qint64 trimStartMillis,
    qint64 trimEndMillis,
    qint64 fadeInMillis,
    qint64 fadeOutMillis,
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    const QVariantMap& restorationValue,
    const QVariantList& equalizerBands,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    const QVariantMap& reverbValue,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis
) {
    if (trimStartMillis < 0 || trimEndMillis <= trimStartMillis || fadeInMillis < 0
        || fadeOutMillis < 0 || fadeInCurve < 0 || fadeInCurve > 2 || fadeOutCurve < 0
        || fadeOutCurve > 2 || gainCentibels < -2400 || gainCentibels > 1200
        || (lowCutHertz != 0 && (lowCutHertz < 20 || lowCutHertz > 240))
        || compressorThresholdCentibels < -6000 || compressorThresholdCentibels > 0
        || compressorRatioTenths < 10 || compressorRatioTenths > 200 || compressorAttackMillis < 1
        || compressorAttackMillis > 200 || compressorReleaseMillis < 20
        || compressorReleaseMillis > 2000 || compressorMakeupCentibels < 0
        || compressorMakeupCentibels > 2400 || limiterCeilingCentibels < -600
        || limiterCeilingCentibels > 0 || limiterReleaseMillis < 20
        || limiterReleaseMillis > 1000) {
        return std::nullopt;
    }
    const auto equalizer = ParametricEqualizerProjection::fromQml(equalizerBands);
    const auto reverb = ReverbProjection::fromQml(reverbValue);
    const auto restoration = RestorationProjection::fromQml(restorationValue);
    if (!equalizer.has_value() || !reverb.has_value() || !restoration.has_value()) {
        return std::nullopt;
    }
    return echo::audio::PlaybackAdjustment{
        .trim_start_millis = static_cast<std::uint64_t>(trimStartMillis),
        .trim_end_millis = static_cast<std::uint64_t>(trimEndMillis),
        .fade_in_millis = static_cast<std::uint64_t>(fadeInMillis),
        .fade_out_millis = static_cast<std::uint64_t>(fadeOutMillis),
        .fade_in_curve = static_cast<echo::audio::FadeCurve>(fadeInCurve),
        .fade_out_curve = static_cast<echo::audio::FadeCurve>(fadeOutCurve),
        .gain_centibels = static_cast<std::int16_t>(gainCentibels),
        .low_cut_hertz = static_cast<std::uint16_t>(lowCutHertz),
        .restoration = *restoration,
        .equalizer = *equalizer,
        .compressor =
            {
                .enabled = compressorEnabled,
                .threshold_centibels = static_cast<std::int16_t>(compressorThresholdCentibels),
                .ratio_tenths = static_cast<std::uint16_t>(compressorRatioTenths),
                .attack_millis = static_cast<std::uint16_t>(compressorAttackMillis),
                .release_millis = static_cast<std::uint16_t>(compressorReleaseMillis),
                .makeup_centibels = static_cast<std::int16_t>(compressorMakeupCentibels),
            },
        .reverb = *reverb,
        .limiter = {
            .enabled = limiterEnabled,
            .ceiling_centibels = static_cast<std::int16_t>(limiterCeilingCentibels),
            .release_millis = static_cast<std::uint16_t>(limiterReleaseMillis),
        },
    };
}
