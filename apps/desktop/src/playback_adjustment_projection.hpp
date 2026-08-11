#pragma once

#include <QVariantList>
#include <QVariantMap>

#include <optional>
#include <vector>

#include "echo/audio/adjustment.hpp"

/// Validates and projects one complete QML adjustment snapshot into the
/// audio-engine contract shared by audition, analysis, and offline render.
class PlaybackAdjustmentProjection {
  public:
    /// Projects the flattened saved adjustment carried by one Library asset.
    [[nodiscard]] static std::optional<echo::audio::PlaybackAdjustment>
    fromAssetMap(const QVariantMap& asset);

    [[nodiscard]] static std::optional<echo::audio::DeHumAdjustment>
    deHumFromQml(const QVariantMap& value);
    [[nodiscard]] static std::optional<echo::audio::DeClickAdjustment>
    deClickFromQml(const QVariantMap& value);
    [[nodiscard]] static std::optional<echo::audio::ChannelRepairAdjustment>
    channelRepairFromQml(const QVariantMap& value);
    [[nodiscard]] static std::optional<std::vector<echo::audio::EditSegment>>
    editSegmentsFromQml(const QVariantList& values, qint64 trimStartMillis, qint64 trimEndMillis);
    [[nodiscard]] static std::optional<std::vector<echo::audio::EffectMask>> effectMasksFromQml(
        const QVariantList& values,
        qint64 trimStartMillis,
        qint64 trimEndMillis,
        const QVariantList& effectChain
    );

    [[nodiscard]] static std::optional<echo::audio::PlaybackAdjustment> fromQml(
        qint64 trimStartMillis,
        qint64 trimEndMillis,
        qint64 fadeInMillis,
        qint64 fadeOutMillis,
        int fadeInCurve,
        int fadeOutCurve,
        int gainCentibels,
        int lowCutHertz,
        const QVariantMap& restoration,
        const QVariantMap& deHum,
        const QVariantMap& deClick,
        const QVariantMap& channelRepair,
        bool equalizerEnabled,
        const QVariantList& equalizerBands,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels,
        const QVariantMap& reverb,
        bool limiterEnabled,
        int limiterCeilingCentibels,
        int limiterReleaseMillis,
        const QVariantList& effectChain,
        const QVariantList& editSegments,
        const QVariantList& effectMasks,
        const QVariantMap& creativeVfx = QVariantMap{}
    );
};
