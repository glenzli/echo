//! Memory membership and material intake; Qt projects Rust-owned collection facts.
#include "desktop_backend.hpp"
#include <QDebug>

QString DesktopBackend::setSoundMembership(
    const QString& id,
    bool memory,
    bool materials,
    const QString& category
) {
    try {
        session_->session_set_sound_membership(
            id.toStdString(),
            memory,
            materials,
            category.toStdString()
        );
        emit projectContentChanged();
        emit assetsChanged();
        return {};
    } catch (const rust::Error& error) {
        return QString::fromUtf8(error.what());
    }
}

QVariantList DesktopBackend::projectMaterials(const QString& assemblyId) const {
    QVariantList result;
    try {
        for (const auto& id : session_->session_project_materials(assemblyId.toStdString()))
            result.append(QString::fromUtf8(id.data(), id.size()));
    } catch (const rust::Error& error) {
        qWarning() << "Cannot read project materials:" << error.what();
    }
    return result;
}

QString DesktopBackend::importMaterial(
    const QUrl& file,
    const QString& assemblyId,
    bool global,
    const QString& category
) {
    if (!file.isLocalFile())
        return tr("Choose a local audio file.");
    try {
        session_->session_queue_material_import(
            file.toLocalFile().toStdString(),
            assemblyId.toStdString(),
            global,
            category.toStdString()
        );
        return {};
    } catch (const rust::Error& error) {
        return QString::fromUtf8(error.what());
    }
}

QVariantMap DesktopBackend::memoryOutputDestination(const QString& assemblyId) const {
    try {
        const auto path = session_->session_memory_output_path(assemblyId.toStdString());
        return {{QStringLiteral("path"), QString::fromUtf8(path.data(), path.size())}};
    } catch (const rust::Error& error) {
        return {{QStringLiteral("error"), QString::fromUtf8(error.what())}};
    }
}
