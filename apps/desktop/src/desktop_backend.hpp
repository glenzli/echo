//! Desktop backend facade: the only Qt-owned bridge to the Rust memory
//! engine. QML never opens SQLite; all Library reads go through this object.

#pragma once

#include <QObject>
#include <QUrl>
#include <QVariantList>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

class DesktopBackend : public QObject {
    Q_OBJECT
    Q_PROPERTY(quint64 assetCount READ assetCount NOTIFY assetsChanged)
    Q_PROPERTY(QString catalogPath READ catalogPath NOTIFY assetsChanged)
    Q_PROPERTY(QString cacheRoot READ cacheRoot NOTIFY assetsChanged)

  public:
    explicit DesktopBackend(
        rust::Box<echo::desktop::LibrarySession> session,
        QObject* parent = nullptr
    );
    ~DesktopBackend() override = default;

    Q_INVOKABLE void refresh();
    Q_INVOKABLE QVariantList listAssets() const;
    Q_INVOKABLE QVariantList listKeywordFacets() const;
    Q_INVOKABLE QVariantList listSmartAlbums() const;
    Q_INVOKABLE QVariantList waveformForAsset(const QString& id) const;
    Q_INVOKABLE QVariantList transcriptsForAsset(const QString& id) const;
    Q_INVOKABLE bool setAssetAffinity(const QString& id, bool liked, int rating);
    Q_INVOKABLE bool setAssetAdjustment(
        const QString& id,
        qlonglong trimStartMillis,
        qlonglong trimEndMillis,
        qlonglong fadeInMillis,
        qlonglong fadeOutMillis,
        int fadeInCurve,
        int fadeOutCurve,
        int gainCentibels,
        int lowCutHertz,
        int eqLowGainCentibels,
        int eqMidGainCentibels,
        int eqHighGainCentibels,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels,
        bool limiterEnabled,
        int limiterCeilingCentibels,
        int limiterReleaseMillis
    );
    Q_INVOKABLE QVariantList search(const QString& query) const;
    Q_INVOKABLE QVariantMap analysisStatusForAsset(const QString& id) const;
    Q_INVOKABLE bool retryAnalysis(const QString& id);
    void startWorkers(const QString& runtimeEndpoint);
    Q_INVOKABLE void queueScans();
    Q_INVOKABLE QVariantMap jobStats() const;
    Q_INVOKABLE QVariantList listRoots() const;
    Q_INVOKABLE bool addRoot(const QUrl& folder);
    Q_INVOKABLE void removeRoot(qlonglong id);
    quint64 assetCount() const;
    QString catalogPath() const;
    QString cacheRoot() const;

  signals:
    void assetsChanged();
    void jobsChanged();

  private:
    rust::Box<echo::desktop::LibrarySession> session_;
};
