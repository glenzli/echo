//! Desktop backend facade: the only Qt-owned bridge to the Rust memory
//! engine. QML never opens SQLite; all Library reads go through this object.
//! Transcription runs on a detached analysis thread through the stateless
//! bridge function and reports back through queued signals.

#pragma once

#include <QObject>
#include <QVariantList>

#include <thread>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

class DesktopBackend : public QObject {
    Q_OBJECT
    Q_PROPERTY(quint64 assetCount READ assetCount NOTIFY assetsChanged)
    Q_PROPERTY(QString catalogPath READ catalogPath NOTIFY assetsChanged)
    Q_PROPERTY(bool transcribing READ transcribing NOTIFY transcriptionStateChanged)

  public:
    explicit DesktopBackend(
        rust::Box<echo::desktop::LibrarySession> session,
        QObject* parent = nullptr
    );
    ~DesktopBackend() override;

    Q_INVOKABLE void refresh();
    Q_INVOKABLE QVariantList listAssets() const;
    Q_INVOKABLE QVariantList waveformForAsset(const QString& id) const;
    Q_INVOKABLE QVariantList transcriptsForAsset(const QString& id) const;
    Q_INVOKABLE void transcribeAsset(const QString& id, const QString& modelRoot,
                                     const QString& python, const QString& workerScript);
    quint64 assetCount() const;
    QString catalogPath() const;
    QString cacheRoot() const;
    bool transcribing() const;

  signals:
    void assetsChanged();
    void transcriptionStateChanged();
    void transcriptionFinished(const QString& assetId, bool ok, const QString& message);

  private:
    rust::Box<echo::desktop::LibrarySession> session_;
    std::thread analysis_thread_;
    bool transcribing_ = false;
};
