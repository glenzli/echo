//! User-owned named snapshots of bounded Creative VFX intent.
//!
//! Presets live in Echo's local application settings, not in an AudioAsset.
//! Applying one remains an ordinary undoable adjustment edit for that asset.

#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>
#include <QVariantMap>

#include <memory>

class QSettings;

class CreativeVfxPresets : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList presets READ presets NOTIFY presetsChanged)

  public:
    explicit CreativeVfxPresets(QObject* parent = nullptr);

    [[nodiscard]] QVariantList presets() const;

    Q_INVOKABLE bool savePreset(
        const QString& name,
        const QVariantMap& creativeVfx,
        const QVariantList& effectChain
    );
    Q_INVOKABLE bool removePreset(const QString& name);

  signals:
    void presetsChanged();

  private:
    [[nodiscard]] QVariantList load() const;
    void store(const QVariantList& values);

    std::unique_ptr<QSettings> settings_;
};
