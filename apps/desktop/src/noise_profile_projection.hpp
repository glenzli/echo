#pragma once

#include "echo/audio/profiled_noise_reduction.hpp"
#include <QVariantMap>
#include <optional>

namespace NoiseProfileProjection {
[[nodiscard]] std::optional<echo::audio::ProfiledNoiseReduction> fromQml(const QVariantMap& value);
[[nodiscard]] QVariantMap toQml(const echo::audio::ProfiledNoiseReduction& value);
} // namespace NoiseProfileProjection
