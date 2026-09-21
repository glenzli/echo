// User-declared source provenance is separate from draft audio adjustments.
#include "desktop_backend.hpp"
#include <QJsonArray>
#include <QJsonDocument>

QString DesktopBackend::setSourceDisclosure(
    const QString& id,
    qlonglong expectedRevision,
    const QVariantList& spans
) {
    try {
        const auto json =
            QJsonDocument(QJsonArray::fromVariantList(spans)).toJson(QJsonDocument::Compact);
        session_
            ->session_set_source_disclosure(id.toStdString(), expectedRevision, json.toStdString());
        emit assetsChanged();
        return {};
    } catch (const rust::Error&) {
        return tr("Source labels could not be saved. They may have changed; reopen and try again.");
    }
}
