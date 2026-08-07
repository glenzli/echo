//! UI preferences: appearance mode with system-following effective theme.
//! Mirrors Shadow's UiPreferences contract (system / light / dark).

#pragma once

#include <QGuiApplication>
#include <QObject>
#include <QSettings>
#include <QString>

#include <memory>

class UiPreferences : public QObject {
    Q_OBJECT
    Q_PROPERTY(int mode READ mode WRITE setMode NOTIFY modeChanged)
    Q_PROPERTY(bool dark READ dark NOTIFY darkChanged)

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

  signals:
    void modeChanged();
    void darkChanged();

  private:
    void refreshEffectiveAppearance();
    void storeMode();

    QGuiApplication& application_;
    std::unique_ptr<QSettings> settings_;
    int mode_ = static_cast<int>(AppearanceMode::System);
    bool effective_dark_ = true;
};