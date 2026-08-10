#include "playback_adjustment_projection.hpp"

#include "parametric_equalizer_projection.hpp"
#include "restoration_projection.hpp"
#include "reverb_projection.hpp"

namespace {

struct EffectChainProjection {
    std::array<echo::audio::EffectNodeKind, echo::audio::kEffectNodeCount> nodes;
    std::uint8_t active_count;
};

std::optional<EffectChainProjection> effectChainFromQml(const QVariantList& values) {
    if (values.isEmpty() || values.size() > static_cast<qsizetype>(echo::audio::kEffectNodeCount)
        || values.back().toInt() != static_cast<int>(echo::audio::EffectNodeKind::Master)) {
        return std::nullopt;
    }
    constexpr std::array standard{
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
    };
    std::array<echo::audio::EffectNodeKind, echo::audio::kEffectNodeCount> result = standard;
    std::array<bool, echo::audio::kEffectNodeCount> seen{};
    for (qsizetype index = 0; index < values.size(); ++index) {
        const int value = values[index].toInt();
        if (value < 0 || value >= static_cast<int>(echo::audio::kEffectNodeCount)
            || seen[static_cast<std::size_t>(value)]) {
            return std::nullopt;
        }
        seen[static_cast<std::size_t>(value)] = true;
        result[static_cast<std::size_t>(index)] = static_cast<echo::audio::EffectNodeKind>(value);
    }
    std::size_t write_index = static_cast<std::size_t>(values.size());
    for (const auto node : standard) {
        if (!seen[static_cast<std::size_t>(node)]) {
            result[write_index++] = node;
        }
    }
    return EffectChainProjection{
        .nodes = result,
        .active_count = static_cast<std::uint8_t>(values.size()),
    };
}

std::optional<echo::audio::DeHumAdjustment> deHumFromQmlImpl(const QVariantMap& value) {
    const int fundamental = value.value(QStringLiteral("fundamentalHertz"), 50).toInt();
    const int harmonics = value.value(QStringLiteral("harmonicCount"), 4).toInt();
    const int quality = value.value(QStringLiteral("qualityTenths"), 300).toInt();
    const int depth = value.value(QStringLiteral("depthCentibels"), 2400).toInt();
    if ((fundamental != 50 && fundamental != 60) || harmonics < 1 || harmonics > 8 || quality < 50
        || quality > 1000 || depth < 0 || depth > 4800) {
        return std::nullopt;
    }
    return echo::audio::DeHumAdjustment{
        .enabled = value.value(QStringLiteral("enabled")).toBool(),
        .fundamental_hertz = static_cast<std::uint16_t>(fundamental),
        .harmonic_count = static_cast<std::uint8_t>(harmonics),
        .quality_tenths = static_cast<std::uint16_t>(quality),
        .depth_centibels = static_cast<std::uint16_t>(depth),
    };
}

std::optional<echo::audio::DeClickAdjustment> deClickFromQmlImpl(const QVariantMap& value) {
    const int sensitivity = value.value(QStringLiteral("sensitivityPercent"), 50).toInt();
    const int maximum_click = value.value(QStringLiteral("maximumClickMicroseconds"), 1000).toInt();
    const int repair = value.value(QStringLiteral("repairPercent"), 100).toInt();
    if (sensitivity < 0 || sensitivity > 100 || maximum_click < 50 || maximum_click > 2000
        || repair < 0 || repair > 100) {
        return std::nullopt;
    }
    return echo::audio::DeClickAdjustment{
        .enabled = value.value(QStringLiteral("enabled")).toBool(),
        .sensitivity_percent = static_cast<std::uint8_t>(sensitivity),
        .maximum_click_microseconds = static_cast<std::uint16_t>(maximum_click),
        .repair_percent = static_cast<std::uint8_t>(repair),
    };
}

} // namespace

std::optional<echo::audio::DeHumAdjustment>
PlaybackAdjustmentProjection::deHumFromQml(const QVariantMap& value) {
    return deHumFromQmlImpl(value);
}

std::optional<echo::audio::DeClickAdjustment>
PlaybackAdjustmentProjection::deClickFromQml(const QVariantMap& value) {
    return deClickFromQmlImpl(value);
}

std::optional<echo::audio::PlaybackAdjustment>
PlaybackAdjustmentProjection::fromAssetMap(const QVariantMap& asset) {
    const QVariantMap restoration{
        {QStringLiteral("enabled"), asset.value(QStringLiteral("restorationEnabled"), true)},
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
    const QVariantMap de_hum{
        {QStringLiteral("enabled"), asset.value(QStringLiteral("deHumEnabled"))},
        {QStringLiteral("fundamentalHertz"),
         asset.value(QStringLiteral("deHumFundamentalHertz"), 50)},
        {QStringLiteral("harmonicCount"), asset.value(QStringLiteral("deHumHarmonicCount"), 4)},
        {QStringLiteral("qualityTenths"), asset.value(QStringLiteral("deHumQualityTenths"), 300)},
        {QStringLiteral("depthCentibels"),
         asset.value(QStringLiteral("deHumDepthCentibels"), 2400)},
    };
    const QVariantMap de_click{
        {QStringLiteral("enabled"), asset.value(QStringLiteral("deClickEnabled"))},
        {QStringLiteral("sensitivityPercent"),
         asset.value(QStringLiteral("deClickSensitivityPercent"), 50)},
        {QStringLiteral("maximumClickMicroseconds"),
         asset.value(QStringLiteral("deClickMaximumClickMicroseconds"), 1000)},
        {QStringLiteral("repairPercent"), asset.value(QStringLiteral("deClickRepairPercent"), 100)},
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
        de_hum,
        de_click,
        asset.value(QStringLiteral("equalizerEnabled"), true).toBool(),
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
        asset.value(QStringLiteral("limiterReleaseMillis")).toInt(),
        asset.value(QStringLiteral("effectChain")).toList()
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
    const QVariantMap& deHumValue,
    const QVariantMap& deClickValue,
    bool equalizerEnabled,
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
    int limiterReleaseMillis,
    const QVariantList& effectChainValue
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
    auto equalizer = ParametricEqualizerProjection::fromQml(equalizerBands);
    const auto reverb = ReverbProjection::fromQml(reverbValue);
    const auto restoration = RestorationProjection::fromQml(restorationValue);
    const auto deHum = deHumFromQml(deHumValue);
    const auto deClick = deClickFromQml(deClickValue);
    const auto effectChain = effectChainFromQml(effectChainValue);
    if (!equalizer.has_value() || !reverb.has_value() || !restoration.has_value()
        || !deHum.has_value() || !deClick.has_value() || !effectChain.has_value()) {
        return std::nullopt;
    }
    equalizer->enabled = equalizerEnabled;
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
        .de_hum = *deHum,
        .de_click = *deClick,
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
        .limiter =
            {
                .enabled = limiterEnabled,
                .ceiling_centibels = static_cast<std::int16_t>(limiterCeilingCentibels),
                .release_millis = static_cast<std::uint16_t>(limiterReleaseMillis),
            },
        .effect_chain = effectChain->nodes,
        .effect_chain_count = effectChain->active_count,
    };
}
