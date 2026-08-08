#include "ui_preferences.hpp"

#include <QDir>
#include <QQmlEngine>
#include <QStandardPaths>
#include <QStyleHints>

namespace {

constexpr auto kAppearanceSettingsKey = "ui/appearanceMode";
constexpr auto kLanguageSettingsKey = "ui/language";

int normalizeMode(int mode) {
    if (mode >= static_cast<int>(UiPreferences::AppearanceMode::System)
        && mode <= static_cast<int>(UiPreferences::AppearanceMode::Dark)) {
        return mode;
    }
    return static_cast<int>(UiPreferences::AppearanceMode::System);
}

QString normalizeLanguageMode(const QString& mode) {
    if (mode == QStringLiteral("zh_CN") || mode == QStringLiteral("en")) {
        return mode;
    }
    return QStringLiteral("system");
}

QString systemLanguage() {
    const QStringList ui_languages = QLocale::system().uiLanguages();
    for (const QString& language : ui_languages) {
        if (language.startsWith(QStringLiteral("zh"))) {
            return QStringLiteral("zh_CN");
        }
    }
    return QStringLiteral("en");
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
    language_mode_ = normalizeLanguageMode(
        settings_->value(QString::fromLatin1(kLanguageSettingsKey), QStringLiteral("system"))
            .toString()
    );
    QObject::connect(
        application_.styleHints(),
        &QStyleHints::colorSchemeChanged,
        this,
        [this](const Qt::ColorScheme) { refreshEffectiveAppearance(); }
    );
    refreshEffectiveAppearance();
    applyLanguage();
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

QString UiPreferences::languageMode() const {
    return language_mode_;
}

void UiPreferences::setLanguageMode(const QString& mode) {
    const QString normalized = normalizeLanguageMode(mode);
    if (normalized == language_mode_) {
        return;
    }
    language_mode_ = normalized;
    settings_->setValue(QString::fromLatin1(kLanguageSettingsKey), language_mode_);
    emit languageModeChanged();
    applyLanguage();
}

void UiPreferences::attachEngine(QQmlEngine& engine) {
    engine_ = &engine;
    engine_->retranslate();
}

void UiPreferences::applyLanguage() {
    const QString effective =
        language_mode_ == QStringLiteral("system") ? systemLanguage() : language_mode_;
    auto next_translator = std::make_unique<QTranslator>();
    bool install_next = false;
    if (effective == QStringLiteral("zh_CN")) {
        install_next = next_translator->load(QStringLiteral(":/i18n/echo_zh_CN.qm"));
        if (!install_next) {
            effective_language_ = QStringLiteral("en");
        }
    }
    if (translator_ != nullptr) {
        QCoreApplication::removeTranslator(translator_.get());
    }
    translator_ = std::move(next_translator);
    if (install_next) {
        QCoreApplication::installTranslator(translator_.get());
    }
    effective_language_ = effective;
    QLocale::setDefault(QLocale(
        effective == QStringLiteral("zh_CN") ? QStringLiteral("zh_CN") : QStringLiteral("en_US")
    ));
    if (engine_ != nullptr) {
        engine_->retranslate();
    }
}
