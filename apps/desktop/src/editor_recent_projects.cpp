#include "editor_recent_projects.hpp"
#include "editor_recovery.hpp"
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLockFile>
#include <QSaveFile>
#include <algorithm>

namespace {
constexpr qsizetype maximumProjects = 40;
QString indexPath(const QString& home) {
    return QDir(home).filePath(QStringLiteral("recent-projects.json"));
}
QJsonArray readProjects(const QString& home, bool* valid = nullptr) {
    QFile file(indexPath(home));
    if (valid)
        *valid = true;
    if (!file.exists())
        return {};
    if (!file.open(QIODevice::ReadOnly)) {
        if (valid)
            *valid = false;
        return {};
    }
    const auto document = QJsonDocument::fromJson(file.readAll());
    if (!document.isArray()) {
        if (valid)
            *valid = false;
        return {};
    }
    return document.array();
}
} // namespace

namespace EditorRecentProjects {
bool remember(const QString& home, const QString& project) {
    const QFileInfo info(project);
    if (!info.isFile() || info.suffix().compare(QStringLiteral("echo"), Qt::CaseInsensitive) != 0
        || !QDir().mkpath(home))
        return false;
    const auto path = info.canonicalFilePath();
    QLockFile lock(indexPath(home) + QStringLiteral(".lock"));
    if (!lock.tryLock(1000))
        return false;
    bool valid = false;
    const auto existing = readProjects(home, &valid);
    if (!valid)
        return false;
    QJsonArray projects{QJsonObject{
        {QStringLiteral("path"), path},
        {QStringLiteral("modifiedMillis"), QDateTime::currentMSecsSinceEpoch()}
    }};
    for (const auto& value : existing) {
        const auto row = value.toObject();
        if (projects.size() >= maximumProjects)
            break;
        if (row.value(QStringLiteral("path")).toString() != path
            && !row.value(QStringLiteral("path")).toString().isEmpty())
            projects.append(row);
    }
    QSaveFile output(indexPath(home));
    if (!output.open(QIODevice::WriteOnly))
        return false;
    const auto bytes = QJsonDocument(projects).toJson(QJsonDocument::Compact);
    return output.write(bytes) == bytes.size() && output.commit();
}

QVariantList list(const QString& home, const QString& currentSession) {
    auto result = EditorRecovery::list(home, currentSession);
    for (auto& value : result) {
        auto row = value.toMap();
        row.insert(QStringLiteral("kind"), QStringLiteral("recovery"));
        value = row;
    }
    for (const auto& value : readProjects(home)) {
        auto row = value.toObject().toVariantMap();
        const QFileInfo project(row.value(QStringLiteral("path")).toString());
        // A disconnected volume may return later; do not erase its history entry.
        if (!project.isFile() || !project.isReadable())
            continue;
        row.insert(QStringLiteral("kind"), QStringLiteral("project"));
        row.insert(QStringLiteral("title"), project.completeBaseName());
        row.insert(QStringLiteral("folder"), project.absolutePath());
        result.append(row);
    }
    std::stable_sort(result.begin(), result.end(), [](const QVariant& left, const QVariant& right) {
        return left.toMap().value(QStringLiteral("modifiedMillis")).toLongLong()
               > right.toMap().value(QStringLiteral("modifiedMillis")).toLongLong();
    });
    return result;
}
} // namespace EditorRecentProjects
