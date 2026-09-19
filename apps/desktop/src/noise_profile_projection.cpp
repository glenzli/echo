#include "noise_profile_projection.hpp"

namespace NoiseProfileProjection {
std::optional<echo::audio::ProfiledNoiseReduction> fromQml(const QVariantMap& value) {
    const auto read = [&](const char* key, qint64 low, qint64 high) -> std::optional<qint64> {
        bool ok = false;
        const auto result = value.value(QString::fromLatin1(key)).toLongLong(&ok);
        return ok && result >= low && result <= high ? std::optional<qint64>{result} : std::nullopt;
    };
    const auto version = read("algorithmVersion", 1, 1);
    const auto start = read("captureStartMillis", 0, 14'400'000);
    const auto end = read("captureEndMillis", 100, 14'400'000);
    const auto reduction = read("reductionCentibels", 0, 3'600);
    const auto sensitivity = read("sensitivityCentibels", 0, 1'200);
    const auto smoothing = read("smoothingBins", 0, 8);
    const auto powers = value.value(QStringLiteral("powerCentibels")).toList();
    if (!version || !start || !end || !reduction || !sensitivity || !smoothing
        || powers.size() != 1'025)
        return std::nullopt;
    echo::audio::ProfiledNoiseReduction result;
    result.enabled = value.value(QStringLiteral("enabled"), false).toBool();
    result.capture_start_millis = static_cast<std::uint64_t>(*start);
    result.capture_end_millis = static_cast<std::uint64_t>(*end);
    result.reduction_centibels = static_cast<std::int16_t>(*reduction);
    result.sensitivity_centibels = static_cast<std::int16_t>(*sensitivity);
    result.smoothing_bins = static_cast<std::uint16_t>(*smoothing);
    for (const auto& value : powers) {
        bool ok = false;
        const auto power = value.toInt(&ok);
        if (!ok || power < -14'400 || power > 1'200)
            return std::nullopt;
        result.power_centibels.push_back(static_cast<std::int16_t>(power));
    }
    try {
        echo::audio::validate_noise_profile(result, result.capture_end_millis);
    } catch (const std::exception&) {
        return std::nullopt;
    }
    return result;
}

QVariantMap toQml(const echo::audio::ProfiledNoiseReduction& value) {
    QVariantList powers;
    powers.reserve(static_cast<qsizetype>(value.power_centibels.size()));
    for (const auto power : value.power_centibels)
        powers.append(power);
    return {
        {QStringLiteral("algorithmVersion"), value.algorithm_version},
        {QStringLiteral("enabled"), value.enabled},
        {QStringLiteral("captureStartMillis"),
         QVariant::fromValue<qulonglong>(value.capture_start_millis)},
        {QStringLiteral("captureEndMillis"),
         QVariant::fromValue<qulonglong>(value.capture_end_millis)},
        {QStringLiteral("powerCentibels"), powers},
        {QStringLiteral("reductionCentibels"), value.reduction_centibels},
        {QStringLiteral("sensitivityCentibels"), value.sensitivity_centibels},
        {QStringLiteral("smoothingBins"), value.smoothing_bins}
    };
}
} // namespace NoiseProfileProjection
