#include "editor_recent_projects.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLockFile>
#include <QTemporaryDir>
#include <QVariantMap>
#include <cassert>

void write(const QString& path, const QByteArray& data) {
    QFile file(path);
    assert(file.open(QIODevice::WriteOnly));
    assert(file.write(data) == data.size());
}
int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir fixture;
    assert(fixture.isValid());
    const QDir root(fixture.path());
    const auto home = root.filePath("isolated-session-home");
    const auto first = root.filePath(QString::fromUtf8("雨声.echo"));
    const auto second = root.filePath("trip.echo");
    write(first, "saved project bytes"); write(second, "another saved project");
    assert(EditorRecentProjects::remember(home, first));
    assert(EditorRecentProjects::remember(home, second));
    assert(EditorRecentProjects::remember(home, first));
    auto recent = EditorRecentProjects::list(home, "");
    assert(recent.size() == 2);
    assert(recent[0].toMap()["title"] == QString::fromUtf8("雨声"));
    assert(recent[0].toMap()["kind"] == "project");
    assert(recent[0].toMap()["path"] == QFileInfo(first).canonicalFilePath());
    assert(EditorRecentProjects::list(home, "") == recent);
    // History does not modify the source project or erase disconnected entries.
    assert(QFile::rename(first, first + ".away"));
    assert(EditorRecentProjects::list(home, "").size() == 1);
    assert(QFile::rename(first + ".away", first));
    assert(EditorRecentProjects::list(home, "").size() == 2);
    QFile source(first); assert(source.open(QIODevice::ReadOnly)); assert(source.readAll() == "saved project bytes");
    const auto index = QDir(home).filePath("recent-projects.json");
    QLockFile lock(index + ".lock"); assert(lock.tryLock());
    assert(!EditorRecentProjects::remember(home, second)); lock.unlock();
    for (int i = 0; i < 45; ++i) {
        const auto path = root.filePath(QString::number(i) + ".echo"); write(path, "project");
        assert(EditorRecentProjects::remember(home, path));
    }
    recent = EditorRecentProjects::list(home, "");
    assert(recent.size() == 40);
    assert(recent[0].toMap()["title"] == "44");
    assert(!EditorRecentProjects::remember(home, root.filePath("missing.echo")));
    write(index, "corrupt history");
    assert(!EditorRecentProjects::remember(home, first));
    QFile preserved(index); assert(preserved.open(QIODevice::ReadOnly)); assert(preserved.readAll() == "corrupt history");
}
