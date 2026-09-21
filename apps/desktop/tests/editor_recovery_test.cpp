#include "editor_recovery.hpp"
#include "echo-desktop-bridge/src/lib.rs.h"
#include <QCoreApplication>
#include <QDataStream>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QLockFile>
#include <QTemporaryDir>
#include <QVariantMap>
#include <cassert>

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir fixture;
    assert(fixture.isValid());
    const QDir base(fixture.path());
    qunsetenv("ECHO_DEBUG_INDEPENDENT_ROOT");
    qputenv("ECHO_DEBUG_EDITOR_SESSION_HOME", base.filePath("manual").toUtf8());
    assert(EditorRecovery::sessionHome() == base.filePath("manual"));
    qputenv("ECHO_DEBUG_INDEPENDENT_ROOT", fixture.path().toUtf8());
    const auto home = base.filePath(".editor-sessions");
    assert(EditorRecovery::sessionHome() == home);
    assert(QDir().mkpath(home));
    const auto wave = base.filePath(QString::fromUtf8("雨声.wav"));
    QFile source(wave);
    assert(source.open(QIODevice::WriteOnly));
    QDataStream stream(&source);
    stream.setByteOrder(QDataStream::LittleEndian);
    stream.writeRawData("RIFF", 4); stream << quint32(36 + 9600);
    stream.writeRawData("WAVEfmt ", 8); stream << quint32(16) << quint16(1) << quint16(1)
        << quint32(48000) << quint32(96000) << quint16(2) << quint16(16);
    stream.writeRawData("data", 4); stream << quint32(9600);
    const QByteArray silence(9600, 0);
    stream.writeRawData(silence.constData(), silence.size());
    source.close();
    const QDir sessions(home);
    const auto older = sessions.filePath("older");
    const auto newer = sessions.filePath("newer");
    const auto active = sessions.filePath("active");
    const auto current = sessions.filePath("current");
    for (const auto& path : {older, newer, active, current}) {
        echo::desktop::editor_import_audio(path.toStdString(), wave.toStdString());
    }
    echo::desktop::open_editor_session(sessions.filePath("empty").toStdString());
    assert(QDir().mkpath(sessions.filePath("foreign")));
    QFile foreign(sessions.filePath("foreign/catalog.sqlite"));
    assert(foreign.open(QIODevice::WriteOnly)); foreign.write("not a catalog"); foreign.close();
    const auto oldTime = QDateTime::fromMSecsSinceEpoch(1700000000000);
    for (const auto& path : {older, newer}) {
        QFile database(QDir(path).filePath("catalog.sqlite"));
        assert(database.open(QIODevice::ReadWrite));
        assert(database.setFileTime(path == older ? oldTime : oldTime.addSecs(600), QFileDevice::FileModificationTime));
    }
    QLockFile lock(QDir(active).filePath("session.lock"));
    assert(lock.tryLock());
    const auto first = EditorRecovery::list(home, current);
    assert(first.size() == 2);
    assert(first[0].toMap()["path"] == newer);
    assert(first[1].toMap()["title"] == QString::fromUtf8("雨声.wav"));
    assert(first[1].toMap()["sourceCount"].toInt() == 1);
    assert(first[1].toMap()["modifiedMillis"].toLongLong() == oldTime.toMSecsSinceEpoch());
    // Discovery creates/removes lock files but cannot change the displayed edit date.
    assert(EditorRecovery::list(home, current) == first);
    lock.unlock();
    assert(EditorRecovery::list(home, current).size() == 3);
    assert(QFile::exists(sessions.filePath("foreign/catalog.sqlite")));
    assert(QFile::exists(sessions.filePath("empty/catalog.sqlite")));
}
