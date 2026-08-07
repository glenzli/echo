#include "desktop_backend.hpp"

DesktopBackend::DesktopBackend(rust::Box<echo::desktop::LibrarySession> session, QObject* parent) :
    QObject(parent), session_(std::move(session)) {}

void DesktopBackend::refresh() {
    emit assetsChanged();
}

QVariantList DesktopBackend::listAssets() const {
    QVariantList list;
    const auto assets = session_->session_list_assets();
    for (const auto& asset : assets) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), QString::fromUtf8(asset.id.data(), asset.id.size()));
        entry.insert(
            QStringLiteral("path"),
            QString::fromUtf8(asset.path.data(), asset.path.size())
        );
        entry.insert(
            QStringLiteral("codec"),
            QString::fromUtf8(asset.codec.data(), asset.codec.size())
        );
        entry.insert(
            QStringLiteral("durationMillis"),
            static_cast<qlonglong>(asset.duration_millis)
        );
        entry.insert(
            QStringLiteral("importedAtMillis"),
            static_cast<qlonglong>(asset.imported_at_millis)
        );
        entry.insert(QStringLiteral("maxLevel"), static_cast<int>(asset.max_level));
        list.append(entry);
    }
    return list;
}

QVariantList DesktopBackend::waveformForAsset(const QString& id) const {
    QVariantList levels;
    const auto artifact = session_->session_waveform_artifact(id.toStdString());
    for (const auto& level : artifact.levels) {
        QVariantList mins;
        QVariantList maxs;
        for (const float sample : level.mins) {
            mins.append(static_cast<double>(sample));
        }
        for (const float sample : level.maxs) {
            maxs.append(static_cast<double>(sample));
        }
        QVariantMap entry;
        entry.insert(
            QStringLiteral("samplesPerBucket"),
            static_cast<int>(level.samples_per_bucket)
        );
        entry.insert(QStringLiteral("mins"), mins);
        entry.insert(QStringLiteral("maxs"), maxs);
        levels.append(entry);
    }
    return levels;
}

quint64 DesktopBackend::assetCount() const {
    return session_->session_asset_count();
}

QString DesktopBackend::catalogPath() const {
    const rust::String path = session_->session_catalog_path();
    return QString::fromUtf8(path.data(), path.size());
}

QString DesktopBackend::cacheRoot() const {
    const rust::String path = session_->session_cache_root();
    return QString::fromUtf8(path.data(), path.size());
}
