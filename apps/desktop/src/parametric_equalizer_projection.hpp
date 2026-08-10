#pragma once

#include <QVariantList>

#include <optional>

#include "echo/audio/adjustment.hpp"

/// Owns the structural mapping between QML band maps and the audio contract.
class ParametricEqualizerProjection {
  public:
    [[nodiscard]] static std::optional<echo::audio::ParametricEqualizerAdjustment>
    fromQml(const QVariantList& values);
    [[nodiscard]] static QVariantList
    toQml(const echo::audio::ParametricEqualizerAdjustment& adjustment);
    [[nodiscard]] static QVariantList responseCurve(
        const echo::audio::ParametricEqualizerAdjustment& adjustment,
        int pointCount,
        std::uint32_t sampleRate = 48000
    );
};
