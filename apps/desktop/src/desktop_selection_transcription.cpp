//! Projection and explicit acceptance of range-scoped inference evidence.
#include "desktop_backend.hpp"
#include <QJsonDocument>
QVariantList DesktopBackend::selectionTranscripts(const QString& id) const {
    try {
        const auto value = session_->session_selection_transcripts(id.toStdString());
        return QJsonDocument::fromJson(
                   QByteArray(value.data(), static_cast<qsizetype>(value.size()))
        )
            .toVariant()
            .toList();
    } catch (const rust::Error&) {
        return {};
    }
}
QString DesktopBackend::acceptSelectionTranscript(const QString& id, const QString& value) {
    try {
        session_->session_accept_selection_transcript(id.toStdString(), value.toStdString());
        emit projectContentChanged();
        emit assetsChanged();
        return {};
    } catch (const rust::Error&) {
        return tr("The transcript could not be saved for this source.");
    }
}
