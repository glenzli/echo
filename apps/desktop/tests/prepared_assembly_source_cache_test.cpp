#include "prepared_assembly_source_cache.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include <cassert>
#include <cstring>

QByteArray wav(qint16 amplitude) {
    QByteArray bytes(44 + 48000 * 4, '\0');
    auto put = [&](int offset, auto value) {
        std::memcpy(bytes.data() + offset, &value, sizeof(value));
    };
    std::memcpy(bytes.data(), "RIFF", 4);
    put(4, quint32(bytes.size() - 8));
    std::memcpy(bytes.data() + 8, "WAVEfmt ", 8);
    put(16, quint32(16));
    put(20, quint16(1));
    put(22, quint16(2));
    put(24, quint32(48000));
    put(28, quint32(192000));
    put(32, quint16(4));
    put(34, quint16(16));
    std::memcpy(bytes.data() + 36, "data", 4);
    put(40, quint32(bytes.size() - 44));
    for (int offset = 44; offset < bytes.size(); offset += 2)
        put(offset, amplitude);
    return bytes;
}
void write(const QString& path, const QByteArray& bytes) {
    QFile file(path);
    assert(file.open(QIODevice::WriteOnly));
    assert(file.write(bytes) == bytes.size());
}
QByteArray read(const QString& path) {
    QFile file(path);
    assert(file.open(QIODevice::ReadOnly));
    return file.readAll();
}
int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir root;
    assert(root.isValid());
    const auto source = root.filePath("source.wav"), directory = root.filePath("cache");
    write(source, wav(5000));
    PreparedAssemblySourceCache cache(directory);
    echo::audio::PlaybackAdjustment adjustment;
    adjustment.trim_end_millis = 1000;
    QVariantMap identity{{"gainCentibels", 0}, {"clipId", "first"}};
    const auto first = cache.prepare(source, identity, adjustment, {});
    assert(!first.reused);
    const auto bytes = read(first.path);
    identity["clipId"] = "duplicate";
    const auto reused = cache.prepare(source, identity, adjustment, {.progress = [](double) {
                                          assert(false && "cache hit must not render");
                                      }});
    assert(reused.reused && first.path == reused.path && bytes == read(reused.path));
    identity["gainCentibels"] = -600;
    adjustment.gain_centibels = -600;
    const auto changed = cache.prepare(source, identity, adjustment, {});
    assert(!changed.reused && changed.path != first.path && bytes != read(changed.path));
    write(source, wav(7000)); // Same path, size and parameters; content must invalidate.
    const auto replaced = cache.prepare(source, identity, adjustment, {});
    assert(!replaced.reused && replaced.path != changed.path);
    write(source, wav(9000));
    bool cancelled = false;
    int checks = 0;
    try {
        cache.prepare(source, identity, adjustment, {.cancelled = [&] { return ++checks > 4; }});
    } catch (const echo::audio::OfflineRenderCancelled&) {
        cancelled = true;
    }
    assert(cancelled);
    assert(QDir(directory).entryList(QDir::Files).size() == 3);
    cache.trim(0, {changed.path});
    assert(QDir(directory).entryList(QDir::Files).size() == 1);
    assert(QFile::exists(changed.path));
    cache.trim(0);
    assert(QDir(directory).entryList(QDir::Files).isEmpty());
}
