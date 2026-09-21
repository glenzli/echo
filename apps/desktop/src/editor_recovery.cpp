#include "editor_recovery.hpp"
#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"
#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLockFile>
#include <QStandardPaths>
#include <algorithm>

namespace EditorRecovery {
QString sessionHome() {
    // Packaged workflow fixtures must never populate the user's recovery history.
    const auto validationRoot = qEnvironmentVariable("ECHO_DEBUG_INDEPENDENT_ROOT");
    if (!validationRoot.isEmpty())
        return QDir(validationRoot).absoluteFilePath(QStringLiteral(".editor-sessions"));
    const auto isolatedHome = qEnvironmentVariable("ECHO_DEBUG_EDITOR_SESSION_HOME");
    if (!isolatedHome.isEmpty())
        return QFileInfo(isolatedHome).absoluteFilePath();
    return QDir(QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation))
        .filePath(QStringLiteral("IndependentEditor"));
}

QVariantList list(const QString& home, const QString& current) {
    QVariantList result;
    const auto directories =
        QDir(home).entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot | QDir::NoSymLinks, QDir::Name);
    for (const auto& directory : directories) {
        const auto path = directory.absoluteFilePath();
        const auto database = QDir(path).filePath(QStringLiteral("catalog.sqlite"));
        if (path == current || !QFileInfo::exists(database))
            continue;
        // The lock changes directory mtime. Read only persisted database/WAL times.
        const auto modified = std::max(
            QFileInfo(database).lastModified(),
            QFileInfo(database + QStringLiteral("-wal")).lastModified()
        );
        QLockFile lock(QDir(path).filePath(QStringLiteral("session.lock")));
        if (!lock.tryLock())
            continue;
        try {
            const auto json = echo::desktop::editor_recovery_summary(path.toStdString());
            if (json.empty())
                continue;
            auto item = QJsonDocument::fromJson(
                            QByteArray(json.data(), static_cast<qsizetype>(json.size()))
            )
                            .object()
                            .toVariantMap();
            if (item.isEmpty())
                continue;
            item.insert(QStringLiteral("path"), path);
            item.insert(QStringLiteral("modifiedMillis"), modified.toMSecsSinceEpoch());
            result.append(item);
        } catch (const rust::Error&) {
            // Unreadable or foreign directories are preserved, never removed by discovery.
        }
    }
    std::sort(result.begin(), result.end(), [](const QVariant& left, const QVariant& right) {
        const auto a = left.toMap(), b = right.toMap();
        const auto at = a.value(QStringLiteral("modifiedMillis")).toLongLong();
        const auto bt = b.value(QStringLiteral("modifiedMillis")).toLongLong();
        return at == bt ? a.value(QStringLiteral("path")).toString()
                              < b.value(QStringLiteral("path")).toString()
                        : at > bt;
    });
    return result;
}
} // namespace EditorRecovery
