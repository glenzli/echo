#include "space_projection.hpp"

#include <string>

std::optional<echo::audio::SpaceAdjustment>
SpaceProjection::fromQml(const QVariantMap& value, echo::audio::ReverbAdjustment algorithmic) {
    const int mode_value = value.value(QStringLiteral("mode"), 0).toInt();
    if (mode_value < 0 || mode_value > 1) {
        return std::nullopt;
    }
    echo::audio::SpaceAdjustment result{
        .mode = static_cast<echo::audio::SpaceMode>(mode_value),
        .algorithmic = algorithmic,
    };
    const int mix = value.value(QStringLiteral("convolutionMixPercent"), 35).toInt();
    const int wet_gain = value.value(QStringLiteral("convolutionWetGainCentibels"), 0).toInt();
    if (mix < 0 || mix > 100 || wet_gain < -2400 || wet_gain > 1200) {
        return std::nullopt;
    }
    result.convolution.import_id =
        value.value(QStringLiteral("impulseResponseImportId")).toString().toStdString();
    result.convolution.source_hash =
        value.value(QStringLiteral("impulseResponseSourceHash")).toString().toStdString();
    result.convolution.prepared_hash =
        value.value(QStringLiteral("impulseResponsePreparedHash")).toString().toStdString();
    result.convolution.adjustment = {
        .enabled = algorithmic.enabled,
        .mix_percent = static_cast<std::uint8_t>(mix),
        .wet_gain_centibels = static_cast<std::int16_t>(wet_gain),
    };
    if (result.mode == echo::audio::SpaceMode::Algorithmic) {
        return result;
    }
    const std::string path =
        value.value(QStringLiteral("impulseResponsePreparedPath")).toString().toStdString();
    if (result.convolution.import_id.empty() || result.convolution.source_hash.empty()
        || result.convolution.prepared_hash.empty() || path.empty()) {
        return std::nullopt;
    }
    result.convolution.prepared_path = path;
    return result;
}
