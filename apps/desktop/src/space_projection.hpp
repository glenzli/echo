#pragma once

#include <QVariantMap>

#include <optional>

#include "echo/audio/adjustment.hpp"

/// Validates the mutually exclusive Space mode and resolves a prepared local
/// impulse response into worker-owned samples before audio execution.
class SpaceProjection {
  public:
    [[nodiscard]] static std::optional<echo::audio::SpaceAdjustment>
    fromQml(const QVariantMap& value, echo::audio::ReverbAdjustment algorithmic);
};
