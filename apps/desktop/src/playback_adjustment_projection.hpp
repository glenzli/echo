#pragma once

#include <QVariantList>
#include <QVariantMap>

#include <optional>

#include "echo/audio/adjustment.hpp"

/// Validates and projects one complete QML adjustment snapshot into the
/// audio-engine contract shared by audition, analysis, and offline render.
class PlaybackAdjustmentProjection {
  public:
    /// Projects the flattened saved adjustment carried by one Library asset.
    [[nodiscard]] static std::optional<echo::audio::PlaybackAdjustment>
    fromAssetMap(const QVariantMap& asset);

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
        const QVariantList& effectChain
    );
};
