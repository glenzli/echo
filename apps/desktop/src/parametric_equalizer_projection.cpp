#include "parametric_equalizer_projection.hpp"

#include <algorithm>
#include <cmath>

#include "echo/audio/parametric_equalizer.hpp"

std::optional<echo::audio::ParametricEqualizerAdjustment>
ParametricEqualizerProjection::fromQml(const QVariantList& values) {
    if (values.size() != static_cast<qsizetype>(echo::audio::kParametricEqualizerBandCount)) {
        return std::nullopt;
    }
    echo::audio::ParametricEqualizerAdjustment adjustment;
    for (qsizetype index = 0; index < values.size(); ++index) {
        const QVariantMap value = values[index].toMap();
        const int filterKind = value.value(QStringLiteral("filterKind")).toInt();
        const int frequencyHertz = value.value(QStringLiteral("frequencyHertz")).toInt();
        const int qHundredths = value.value(QStringLiteral("qHundredths")).toInt();
        const int gainCentibels = value.value(QStringLiteral("gainCentibels")).toInt();
        if (filterKind < 0 || filterKind > 3 || frequencyHertz < 20 || frequencyHertz > 20000
            || qHundredths < 10 || qHundredths > 2000 || gainCentibels < -1200
            || gainCentibels > 1200) {
            return std::nullopt;
        }
        adjustment.bands[static_cast<std::size_t>(index)] = {
            value.value(QStringLiteral("enabled")).toBool(),
            static_cast<echo::audio::EqualizerFilterKind>(filterKind),
            static_cast<std::uint16_t>(frequencyHertz),
            static_cast<std::uint16_t>(qHundredths),
            static_cast<std::int16_t>(gainCentibels),
        };
    }
    return adjustment;
}

QVariantList
ParametricEqualizerProjection::toQml(const echo::audio::ParametricEqualizerAdjustment& adjustment) {
    QVariantList result;
    for (const auto& band : adjustment.bands) {
        QVariantMap value;
        value.insert(QStringLiteral("enabled"), band.enabled);
        value.insert(QStringLiteral("filterKind"), static_cast<int>(band.filter_kind));
        value.insert(QStringLiteral("frequencyHertz"), static_cast<int>(band.frequency_hertz));
        value.insert(QStringLiteral("qHundredths"), static_cast<int>(band.q_hundredths));
        value.insert(QStringLiteral("gainCentibels"), static_cast<int>(band.gain_centibels));
        result.append(value);
    }
    return result;
}

QVariantList ParametricEqualizerProjection::responseCurve(
    const echo::audio::ParametricEqualizerAdjustment& adjustment,
    int pointCount,
    std::uint32_t sampleRate
) {
    QVariantList result;
    const int count = std::clamp(pointCount, 16, 256);
    constexpr double minimumFrequency = 20.0;
    constexpr double maximumFrequency = 20000.0;
    const double span = std::log(maximumFrequency / minimumFrequency);
    for (int index = 0; index < count; ++index) {
        const double progress = static_cast<double>(index) / static_cast<double>(count - 1);
        const double frequency = minimumFrequency * std::exp(span * progress);
        result.append(
            echo::audio::ParametricEqualizer::response_decibels(adjustment, sampleRate, frequency)
        );
    }
    return result;
}
