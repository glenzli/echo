#pragma once

#include <QByteArray>
#include <QVariantMap>

#include <optional>

#include "echo/audio/creative_vfx.hpp"

/// Exhaustive Qt projection for the four typed Creative VFX families.
class CreativeVfxProjection {
  public:
    [[nodiscard]] static std::optional<echo::audio::CreativeVfxAdjustment>
    fromQml(const QVariantMap& value);
    [[nodiscard]] static std::optional<echo::audio::CreativeVfxAdjustment>
    fromJson(const QByteArray& encoded);
    [[nodiscard]] static QVariantMap toQml(const echo::audio::CreativeVfxAdjustment& adjustment);
    [[nodiscard]] static QByteArray toJson(const echo::audio::CreativeVfxAdjustment& adjustment);
};
