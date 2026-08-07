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

quint64 DesktopBackend::assetCount() const {
    return session_->session_asset_count();
}

QString DesktopBackend::catalogPath() const {
    const rust::String path = session_->session_catalog_path();
    return QString::fromUtf8(path.data(), path.size());
}
