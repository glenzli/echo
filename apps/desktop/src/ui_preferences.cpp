#include "ui_preferences.hpp"

#include <QDir>
#include <QStandardPaths>
#include <QStyleHints>

namespace {

constexpr auto kAppearanceSettingsKey = "ui/appearanceMode";

int normalizeMode(int mode) {
    if (mode >= static_cast<int>(UiPreferences::AppearanceMode::System)
        && mode <= static_cast<int>(UiPreferences::AppearanceMode::Dark)) {
        return mode;
    }
    return static_cast<int>(UiPreferences::AppearanceMode::System);
}

} // namespace

UiPreferences::UiPreferences(QGuiApplication& application, QObject* parent) :
    QObject(parent), application_(application) {
    // INI format with an explicit file: deterministic across platforms and
    // immune to macOS cfprefsd read/write caching for unbundled binaries.
    const QString config_dir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(config_dir);
    settings_ = std::make_unique<QSettings>(
        config_dir + QStringLiteral("/echo.conf"),
        QSettings::IniFormat
    );
    mode_ = normalizeMode(settings_->value(QString::fromLatin1(kAppearanceSettingsKey), 0).toInt());
    QObject::connect(
        application_.styleHints(),
        &QStyleHints::colorSchemeChanged,
        this,
        [this](const Qt::ColorScheme) { refreshEffectiveAppearance(); }
    );
    refreshEffectiveAppearance();
}

int UiPreferences::mode() const {
    return mode_;
}

void UiPreferences::setMode(int mode) {
    const int normalized = normalizeMode(mode);
    if (normalized == mode_) {
        return;
    }
    mode_ = normalized;
    storeMode();
    refreshEffectiveAppearance();
    emit modeChanged();
}

bool UiPreferences::dark() const {
    return effective_dark_;
}

void UiPreferences::refreshEffectiveAppearance() {
    bool dark = false;
    switch (static_cast<AppearanceMode>(mode_)) {
    case AppearanceMode::Light:
        dark = false;
        break;
    case AppearanceMode::Dark:
        dark = true;
        break;
    case AppearanceMode::System: {
        const auto scheme = application_.styleHints()->colorScheme();
        dark = scheme == Qt::ColorScheme::Dark
               || (scheme == Qt::ColorScheme::Unknown
                   && application_.styleHints()->colorScheme() != Qt::ColorScheme::Light);
        break;
    }
    }
    if (dark != effective_dark_) {
        effective_dark_ = dark;
        emit darkChanged();
    }
}

void UiPreferences::storeMode() {
    settings_->setValue(QString::fromLatin1(kAppearanceSettingsKey), mode_);
}
