#pragma once

#include <QVariantMap>

#include <optional>

#include "echo/audio/adjustment.hpp"

class RestorationProjection {
  public:
    [[nodiscard]] static std::optional<echo::audio::RestorationAdjustment>
    fromQml(const QVariantMap& value);
};
