#include "playback_adjustment_projection.hpp"

#include "creative_vfx_projection.hpp"
#include "parametric_equalizer_projection.hpp"
#include "restoration_projection.hpp"
#include "reverb_projection.hpp"
#include "space_projection.hpp"

#include <utility>

namespace {

std::optional<std::vector<echo::audio::SpectralAttenuationRegion>>
spectralRepairFromQml(const QVariantMap& value) {
    if (!value.value(QStringLiteral("enabled"), true).toBool()) {
        return std::vector<echo::audio::SpectralAttenuationRegion>{};
    }
    const QVariantList values = value.value(QStringLiteral("regions")).toList();
    if (values.size() > 64) {
        return std::nullopt;
    }
    std::vector<echo::audio::SpectralAttenuationRegion> regions;
    regions.reserve(values.size());
    for (const QVariant& item : values) {
        const QVariantMap region = item.toMap();
        const qint64 start = region.value(QStringLiteral("startMillis")).toLongLong();
        const qint64 end = region.value(QStringLiteral("endMillis")).toLongLong();
        const int low = region.value(QStringLiteral("lowHertz")).toInt();
        const int high = region.value(QStringLiteral("highHertz")).toInt();
        const int attenuation = region.value(QStringLiteral("attenuationCentibels")).toInt();
        const int time_feather = region.value(QStringLiteral("timeFeatherMillis")).toInt();
        const int frequency_feather = region.value(QStringLiteral("frequencyFeatherHertz")).toInt();
        if (start < 0 || end <= start || low < 20 || high <= low || high > 24'000 || attenuation < 0
            || attenuation > 9'600 || time_feather < 0 || time_feather > 250
            || frequency_feather < 0 || frequency_feather > 2'000) {
            return std::nullopt;
        }
        regions.push_back({
            .start_millis = static_cast<std::uint64_t>(start),
            .end_millis = static_cast<std::uint64_t>(end),
            .low_hertz = static_cast<std::uint16_t>(low),
            .high_hertz = static_cast<std::uint16_t>(high),
            .attenuation_centibels = static_cast<std::int16_t>(attenuation),
            .time_feather_millis = static_cast<std::uint16_t>(time_feather),
            .frequency_feather_hertz = static_cast<std::uint16_t>(frequency_feather),
        });
    }
    return regions;
}

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
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
        echo::audio::EffectNodeKind::TapeVfx,
        echo::audio::EffectNodeKind::PitchVfx,
        echo::audio::EffectNodeKind::AutoWahVfx,
        echo::audio::EffectNodeKind::StereoVfx,
        echo::audio::EffectNodeKind::BeatRepeatVfx,
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

std::optional<std::vector<echo::audio::EditSegment>>
editSegmentsFromQmlImpl(const QVariantList& values, qint64 trimStartMillis, qint64 trimEndMillis) {
    if (trimStartMillis < 0 || trimEndMillis <= trimStartMillis || values.size() > 128) {
        return std::nullopt;
    }
    if (values.isEmpty()) {
        return std::vector<echo::audio::EditSegment>{{
            .source_start_millis = static_cast<std::uint64_t>(trimStartMillis),
            .source_end_millis = static_cast<std::uint64_t>(trimEndMillis),
        }};
    }
    std::vector<echo::audio::EditSegment> result;
    result.reserve(static_cast<std::size_t>(values.size()));
    qint64 expected_start = trimStartMillis;
    std::uint64_t output_millis = 0;
    for (const QVariant& item : values) {
        const QVariantMap value = item.toMap();
        const qint64 start = value.value(QStringLiteral("sourceStartMillis")).toLongLong();
        const qint64 end = value.value(QStringLiteral("sourceEndMillis")).toLongLong();
        const int state = value.value(QStringLiteral("state")).toInt();
        const int gain = value.value(QStringLiteral("gainCentibels")).toInt();
        const qint64 fade_in = value.value(QStringLiteral("fadeInMillis")).toLongLong();
        const qint64 fade_out = value.value(QStringLiteral("fadeOutMillis")).toLongLong();
        const int fade_in_curve = value.value(QStringLiteral("fadeInCurve")).toInt();
        const int fade_out_curve = value.value(QStringLiteral("fadeOutCurve")).toInt();
        const qint64 gap_after = value.value(QStringLiteral("gapAfterMillis")).toLongLong();
        if (start != expected_start || end <= start || end > trimEndMillis || state < 0 || state > 2
            || gain < -2400 || gain > 1200 || fade_in < 0 || fade_out < 0
            || fade_in + fade_out > end - start || fade_in_curve < 0 || fade_in_curve > 2
            || fade_out_curve < 0 || fade_out_curve > 2 || gap_after < 0 || gap_after > 3'600'000) {
            return std::nullopt;
        }
        result.push_back({
            .source_start_millis = static_cast<std::uint64_t>(start),
            .source_end_millis = static_cast<std::uint64_t>(end),
            .state = static_cast<echo::audio::EditSegmentState>(state),
            .gain_centibels = static_cast<std::int16_t>(gain),
            .fade_in_millis = static_cast<std::uint64_t>(fade_in),
            .fade_out_millis = static_cast<std::uint64_t>(fade_out),
            .fade_in_curve = static_cast<echo::audio::FadeCurve>(fade_in_curve),
            .fade_out_curve = static_cast<echo::audio::FadeCurve>(fade_out_curve),
            .gap_after_millis = static_cast<std::uint64_t>(gap_after),
        });
        if (state != static_cast<int>(echo::audio::EditSegmentState::Hidden)) {
            output_millis += static_cast<std::uint64_t>(end - start);
        }
        output_millis += static_cast<std::uint64_t>(gap_after);
        expected_start = end;
    }
    if (expected_start != trimEndMillis || output_millis == 0) {
        return std::nullopt;
    }
    return result;
}

std::optional<std::vector<echo::audio::EffectMask>> effectMasksFromQmlImpl(
    const QVariantList& values,
    qint64 trimStartMillis,
    qint64 trimEndMillis,
    const EffectChainProjection& chain
) {
    if (values.size() > 64) {
        return std::nullopt;
    }
    std::array<bool, echo::audio::kEffectNodeCount> active{};
    for (std::size_t index = 0; index < chain.active_count; ++index) {
        active[static_cast<std::size_t>(chain.nodes[index])] = true;
    }
    std::vector<echo::audio::EffectMask> result;
    result.reserve(static_cast<std::size_t>(values.size()));
    for (const QVariant& item : values) {
        const QVariantMap value = item.toMap();
        const qint64 start = value.value(QStringLiteral("startMillis")).toLongLong();
        const qint64 end = value.value(QStringLiteral("endMillis")).toLongLong();
        const qint64 feather = value.value(QStringLiteral("featherMillis"), 10).toLongLong();
        const QVariantList nodes = value.value(QStringLiteral("effectNodes")).toList();
        if (start < trimStartMillis || end > trimEndMillis || end <= start || feather < 0
            || feather > 100 || nodes.isEmpty()
            || nodes.size() > static_cast<qsizetype>(echo::audio::kEffectNodeCount - 3)) {
            return std::nullopt;
        }
        echo::audio::EffectMask mask{
            .start_millis = static_cast<std::uint64_t>(start),
            .end_millis = static_cast<std::uint64_t>(end),
            .feather_millis = static_cast<std::uint64_t>(feather),
        };
        std::array<bool, echo::audio::kEffectNodeCount> seen{};
        for (const QVariant& node_value : nodes) {
            const int node = node_value.toInt();
            if (node < 0 || node >= static_cast<int>(echo::audio::kEffectNodeCount) || node == 4
                || node == 6 || node == 11 || node == 13 || node == 14 || node == 18 || node == 21
                || !active[static_cast<std::size_t>(node)]
                || seen[static_cast<std::size_t>(node)]) {
                return std::nullopt;
            }
            seen[static_cast<std::size_t>(node)] = true;
            mask.nodes.push_back(static_cast<echo::audio::EffectNodeKind>(node));
        }
        result.push_back(std::move(mask));
    }
    return result;
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

std::optional<echo::audio::ChannelRepairAdjustment>
channelRepairFromQmlImpl(const QVariantMap& value) {
    const int balance = value.value(QStringLiteral("balancePercent"), 0).toInt();
    if (balance < -100 || balance > 100) {
        return std::nullopt;
    }
    return echo::audio::ChannelRepairAdjustment{
        .enabled = value.value(QStringLiteral("enabled")).toBool(),
        .invert_left = value.value(QStringLiteral("invertLeft")).toBool(),
        .invert_right = value.value(QStringLiteral("invertRight")).toBool(),
        .swap_channels = value.value(QStringLiteral("swapChannels")).toBool(),
        .mono_fold_down = value.value(QStringLiteral("monoFoldDown")).toBool(),
        .balance_percent = static_cast<std::int16_t>(balance),
    };
}

} // namespace

std::optional<std::vector<echo::audio::EditSegment>>
PlaybackAdjustmentProjection::editSegmentsFromQml(
    const QVariantList& values,
    qint64 trimStartMillis,
    qint64 trimEndMillis
) {
    return editSegmentsFromQmlImpl(values, trimStartMillis, trimEndMillis);
}

std::optional<std::vector<echo::audio::EffectMask>>
PlaybackAdjustmentProjection::effectMasksFromQml(
    const QVariantList& values,
    qint64 trimStartMillis,
    qint64 trimEndMillis,
    const QVariantList& effectChainValue
) {
    const auto chain = effectChainFromQml(effectChainValue);
    if (!chain.has_value()) {
        return std::nullopt;
    }
    return effectMasksFromQmlImpl(values, trimStartMillis, trimEndMillis, *chain);
}

std::optional<echo::audio::DeHumAdjustment>
PlaybackAdjustmentProjection::deHumFromQml(const QVariantMap& value) {
    return deHumFromQmlImpl(value);
}

std::optional<echo::audio::DeClickAdjustment>
PlaybackAdjustmentProjection::deClickFromQml(const QVariantMap& value) {
    return deClickFromQmlImpl(value);
}

std::optional<echo::audio::ChannelRepairAdjustment>
PlaybackAdjustmentProjection::channelRepairFromQml(const QVariantMap& value) {
    return channelRepairFromQmlImpl(value);
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
    const QVariantMap channel_repair{
        {QStringLiteral("enabled"), asset.value(QStringLiteral("channelRepairEnabled"))},
        {QStringLiteral("invertLeft"), asset.value(QStringLiteral("channelRepairInvertLeft"))},
        {QStringLiteral("invertRight"), asset.value(QStringLiteral("channelRepairInvertRight"))},
        {QStringLiteral("swapChannels"), asset.value(QStringLiteral("channelRepairSwapChannels"))},
        {QStringLiteral("monoFoldDown"), asset.value(QStringLiteral("channelRepairMonoFoldDown"))},
        {QStringLiteral("balancePercent"),
         asset.value(QStringLiteral("channelRepairBalancePercent"), 0)},
    };
    const QVariantMap reverb{
        {QStringLiteral("character"), asset.value(QStringLiteral("reverbCharacter"), 0)},
        {QStringLiteral("enabled"), asset.value(QStringLiteral("reverbEnabled"))},
        {QStringLiteral("mixPercent"), asset.value(QStringLiteral("reverbMixPercent"))},
        {QStringLiteral("preDelayMillis"), asset.value(QStringLiteral("reverbPreDelayMillis"))},
        {QStringLiteral("decayMillis"), asset.value(QStringLiteral("reverbDecayMillis"))},
        {QStringLiteral("sizePercent"), asset.value(QStringLiteral("reverbSizePercent"))},
        {QStringLiteral("dampingPercent"), asset.value(QStringLiteral("reverbDampingPercent"))},
        {QStringLiteral("lowCutHertz"), asset.value(QStringLiteral("reverbLowCutHertz"))},
        {QStringLiteral("highCutHertz"), asset.value(QStringLiteral("reverbHighCutHertz"))},
        {QStringLiteral("ducking"),
         QVariantMap{
             {QStringLiteral("enabled"),
              asset.value(QStringLiteral("reverbDuckingEnabled"), false)},
             {QStringLiteral("amountPercent"),
              asset.value(QStringLiteral("reverbDuckingAmountPercent"), 65)},
             {QStringLiteral("attackMillis"),
              asset.value(QStringLiteral("reverbDuckingAttackMillis"), 10)},
             {QStringLiteral("releaseMillis"),
              asset.value(QStringLiteral("reverbDuckingReleaseMillis"), 250)},
         }},
        {QStringLiteral("mode"), asset.value(QStringLiteral("spaceMode"), 0)},
        {QStringLiteral("impulseResponseImportId"),
         asset.value(QStringLiteral("impulseResponseImportId"))},
        {QStringLiteral("impulseResponseSourceHash"),
         asset.value(QStringLiteral("impulseResponseSourceHash"))},
        {QStringLiteral("impulseResponsePreparedHash"),
         asset.value(QStringLiteral("impulseResponsePreparedHash"))},
        {QStringLiteral("impulseResponsePreparedPath"),
         asset.value(QStringLiteral("impulseResponsePreparedPath"))},
        {QStringLiteral("convolutionMixPercent"),
         asset.value(QStringLiteral("convolutionMixPercent"), 35)},
        {QStringLiteral("convolutionWetGainCentibels"),
         asset.value(QStringLiteral("convolutionWetGainCentibels"), 0)},
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
        channel_repair,
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
        asset.value(QStringLiteral("effectChain")).toList(),
        asset.value(QStringLiteral("editSegments")).toList(),
        asset.value(QStringLiteral("effectMasks")).toList(),
        asset.value(QStringLiteral("creativeVfx")).toMap(),
        asset.value(QStringLiteral("spectralRepair")).toMap()
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
    const QVariantMap& channelRepairValue,
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
    const QVariantList& effectChainValue,
    const QVariantList& editSegmentsValue,
    const QVariantList& effectMasksValue,
    const QVariantMap& creativeVfxValue,
    const QVariantMap& spectralRepairValue
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
    const auto channelRepair = channelRepairFromQml(channelRepairValue);
    const auto creativeVfx = CreativeVfxProjection::fromQml(creativeVfxValue);
    const auto spectralRepair = spectralRepairFromQml(spectralRepairValue);
    const auto effectChain = effectChainFromQml(effectChainValue);
    auto editSegments = editSegmentsFromQmlImpl(editSegmentsValue, trimStartMillis, trimEndMillis);
    const auto space =
        reverb.has_value() ? SpaceProjection::fromQml(reverbValue, *reverb) : std::nullopt;
    if (!equalizer.has_value() || !reverb.has_value() || !space.has_value()
        || !restoration.has_value() || !deHum.has_value() || !deClick.has_value()
        || !channelRepair.has_value() || !creativeVfx.has_value() || !spectralRepair.has_value()
        || !effectChain.has_value() || !editSegments.has_value()) {
        return std::nullopt;
    }
    auto effectMasks =
        effectMasksFromQmlImpl(effectMasksValue, trimStartMillis, trimEndMillis, *effectChain);
    if (!effectMasks.has_value()) {
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
        .channel_repair = *channelRepair,
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
        .space = *space,
        .creative_vfx = *creativeVfx,
        .spectral_repair = std::move(*spectralRepair),
        .limiter =
            {
                .enabled = limiterEnabled,
                .ceiling_centibels = static_cast<std::int16_t>(limiterCeilingCentibels),
                .release_millis = static_cast<std::uint16_t>(limiterReleaseMillis),
            },
        .effect_chain = effectChain->nodes,
        .effect_chain_count = effectChain->active_count,
        .edit_segments = std::move(*editSegments),
        .effect_masks = std::move(*effectMasks),
    };
}
