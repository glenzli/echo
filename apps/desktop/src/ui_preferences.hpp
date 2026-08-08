//! UI preferences: appearance mode with system-following effective theme.
//! Mirrors Shadow's UiPreferences contract (system / light / dark).

#pragma once

#include <QGuiApplication>
#include <QObject>
#include <QQmlEngine>
#include <QSettings>
#include <QString>
#include <QTranslator>

#include <memory>

class UiPreferences : public QObject {
    Q_OBJECT
    Q_PROPERTY(int mode READ mode WRITE setMode NOTIFY modeChanged)
    Q_PROPERTY(bool dark READ dark NOTIFY darkChanged)
    Q_PROPERTY(
        QString languageMode READ languageMode WRITE setLanguageMode NOTIFY languageModeChanged
    )

  public:
    enum class AppearanceMode {
        System = 0,
        Light = 1,
        Dark = 2,
    };
    Q_ENUM(AppearanceMode)

    explicit UiPreferences(QGuiApplication& application, QObject* parent = nullptr);

    int mode() const;
    void setMode(int mode);

    /// Effective dark state: explicit mode, or the platform scheme in System
    /// mode.
    bool dark() const;

    /// UI language: `system`, `zh_CN`, or `en`.
    QString languageMode() const;
    void setLanguageMode(const QString& mode);

    /// Attaches the QML engine for live retranslation.
    void attachEngine(QQmlEngine& engine);

  signals:
    void modeChanged();
    void darkChanged();
    void languageModeChanged();

  private:
    void refreshEffectiveAppearance();
    void applyLanguage();
    void storeMode();

    QGuiApplication& application_;
    std::unique_ptr<QSettings> settings_;
    QQmlEngine* engine_ = nullptr;
    std::unique_ptr<QTranslator> translator_;
    int mode_ = static_cast<int>(AppearanceMode::System);
    bool effective_dark_ = true;
    QString language_mode_ = QStringLiteral("system");
    QString effective_language_ = QStringLiteral("en");
};