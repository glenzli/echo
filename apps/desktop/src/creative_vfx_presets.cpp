#include "creative_vfx_presets.hpp"

#include "creative_vfx_projection.hpp"

#include <QDir>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSettings>
#include <QStandardPaths>

#include <array>

namespace {

constexpr auto kSettingsKey = "creativeVfx/namedPresetsV1";
constexpr qsizetype kMaximumPresetCount = 32;
constexpr qsizetype kMaximumNameLength = 48;

bool validEffectChain(const QVariantList& values) {
    if (values.isEmpty() || values.size() > 22 || values.constLast().toInt() != 4) {
        return false;
    }
    std::array<bool, 22> seen{};
    for (const QVariant& value : values) {
        bool okay = false;
        const int kind = value.toInt(&okay);
        if (!okay || kind < 0 || kind >= static_cast<int>(seen.size()) || seen[kind]) {
            return false;
        }
        seen[kind] = true;
    }
    return true;
}

QVariantList decodedPresets(const QSettings& settings) {
    const QJsonDocument document =
        QJsonDocument::fromJson(settings.value(QString::fromLatin1(kSettingsKey)).toByteArray());
    if (!document.isArray()) {
        return {};
    }
    QVariantList result;
    for (const QJsonValue& value : document.array()) {
        const QVariantMap preset = value.toObject().toVariantMap();
        const QString name = preset.value(QStringLiteral("name")).toString().trimmed();
        const QVariantMap creative = preset.value(QStringLiteral("creativeVfx")).toMap();
        const QVariantList chain = preset.value(QStringLiteral("effectChain")).toList();
        if (name.isEmpty() || name.size() > kMaximumNameLength || !validEffectChain(chain)
            || !CreativeVfxProjection::fromQml(creative).has_value()) {
            continue;
        }
        result.append(
            QVariantMap{
                {QStringLiteral("name"), name},
                {QStringLiteral("creativeVfx"), creative},
                {QStringLiteral("effectChain"), chain},
            }
        );
        if (result.size() == kMaximumPresetCount) {
            break;
        }
    }
    return result;
}

} // namespace

CreativeVfxPresets::CreativeVfxPresets(QObject* parent) : QObject(parent) {
    const QString config_dir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(config_dir);
    settings_ = std::make_unique<QSettings>(
        config_dir + QStringLiteral("/echo.conf"),
        QSettings::IniFormat
    );
}

QVariantList CreativeVfxPresets::load() const {
    return decodedPresets(*settings_);
}

QVariantList CreativeVfxPresets::presets() const {
    return load();
}

void CreativeVfxPresets::store(const QVariantList& values) {
    settings_->setValue(
        QString::fromLatin1(kSettingsKey),
        QJsonDocument::fromVariant(values).toJson(QJsonDocument::Compact)
    );
}

bool CreativeVfxPresets::savePreset(
    const QString& requested_name,
    const QVariantMap& creative_vfx,
    const QVariantList& effect_chain
) {
    const QString name = requested_name.trimmed();
    const auto canonical = CreativeVfxProjection::fromQml(creative_vfx);
    if (name.isEmpty() || name.size() > kMaximumNameLength || !canonical.has_value()
        || !validEffectChain(effect_chain)) {
        return false;
    }
    QVariantList values = load();
    const QVariantMap stored{
        {QStringLiteral("name"), name},
        {QStringLiteral("creativeVfx"), CreativeVfxProjection::toQml(*canonical)},
        {QStringLiteral("effectChain"), effect_chain},
    };
    for (QVariant& value : values) {
        if (value.toMap().value(QStringLiteral("name")).toString() == name) {
            value = stored;
            store(values);
            emit presetsChanged();
            return true;
        }
    }
    if (values.size() >= kMaximumPresetCount) {
        return false;
    }
    values.append(stored);
    store(values);
    emit presetsChanged();
    return true;
}

bool CreativeVfxPresets::removePreset(const QString& requested_name) {
    const QString name = requested_name.trimmed();
    QVariantList values = load();
    for (qsizetype index = 0; index < values.size(); ++index) {
        if (values[index].toMap().value(QStringLiteral("name")).toString() == name) {
            values.removeAt(index);
            store(values);
            emit presetsChanged();
            return true;
        }
    }
    return false;
}
