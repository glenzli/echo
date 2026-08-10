#pragma once

#include <QVariantMap>

#include <optional>

#include "echo/audio/adjustment.hpp"

class ReverbProjection {
  public:
    [[nodiscard]] static std::optional<echo::audio::ReverbAdjustment>
    fromQml(const QVariantMap& value);
};
