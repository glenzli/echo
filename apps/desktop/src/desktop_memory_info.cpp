// Personal context is saved in Echo's Catalog, never in the original file.
#include "desktop_backend.hpp"
#include <QJsonDocument>
#include <QJsonObject>
#include <stdexcept>

QVariantMap DesktopBackend::exportMemoryInfo(const QString& id, bool assembly) const {
    const auto json = session_->session_memory_info(id.toStdString(), assembly);
    return QJsonDocument::fromJson(QByteArray(json.data(), json.size())).object().toVariantMap();
}
QVariantMap DesktopBackend::memoryInfo(const QString& id, bool assembly) const {
    try {
        return exportMemoryInfo(id, assembly);
    } catch (const rust::Error&) {
        return {{QStringLiteral("error"), tr("Memory information could not be loaded.")}};
    }
}
QString DesktopBackend::setMemoryInfo(
    const QString& id,
    bool assembly,
    qlonglong expectedRevision,
    const QVariantMap& info
) {
    try {
        const auto json =
            QJsonDocument(QJsonObject::fromVariantMap(info)).toJson(QJsonDocument::Compact);
        session_->session_set_memory_info(
            id.toStdString(),
            assembly,
            expectedRevision,
            json.toStdString()
        );
        emit assetsChanged();
        return {};
    } catch (const rust::Error&) {
        return tr(
            "Memory information could not be saved. Check the text and time ranges; if another "
            "window changed it, reopen before saving."
        );
    }
}
