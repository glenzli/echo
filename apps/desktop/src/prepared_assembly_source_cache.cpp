#include "prepared_assembly_source_cache.hpp"
#include "echo/audio/offline_wav_renderer.hpp"
#include "qt_render_byte_sink.hpp"
#include <QCryptographicHash>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QSaveFile>
#include <stdexcept>
#include <utility>

namespace {
QByteArray sourceDigest(const QString& path, const echo::audio::OfflineRenderCallbacks& callbacks) {
    QFile input(path);
    if (!input.open(QIODevice::ReadOnly))
        throw std::runtime_error(input.errorString().toStdString());
    QCryptographicHash hash(QCryptographicHash::Sha256);
    while (!input.atEnd()) {
        if (callbacks.cancelled && callbacks.cancelled())
            throw echo::audio::OfflineRenderCancelled();
        const auto bytes = input.read(1024 * 1024);
        if (bytes.isEmpty() && input.error() != QFile::NoError)
            throw std::runtime_error(input.errorString().toStdString());
        hash.addData(bytes);
    }
    return hash.result();
}
} // namespace

PreparedAssemblySourceCache::PreparedAssemblySourceCache(QString root) : root_(std::move(root)) {}

PreparedAssemblySourceCache::Result PreparedAssemblySourceCache::prepare(
    const QString& source,
    QVariantMap identity,
    const echo::audio::PlaybackAdjustment& adjustment,
    const echo::audio::OfflineRenderCallbacks& callbacks
) {
    if (!QDir().mkpath(root_))
        throw std::runtime_error("cannot create prepared source cache");
    identity.remove(QStringLiteral("clipId"));
    QCryptographicHash key(QCryptographicHash::Sha256);
    key.addData(QByteArrayLiteral("echo-prepared-assembly-pcm24-v1"));
    const auto before = sourceDigest(source, callbacks);
    key.addData(before);
    const auto impulsePath =
        identity.value(QStringLiteral("impulseResponsePreparedPath")).toString();
    if (!impulsePath.isEmpty())
        key.addData(sourceDigest(impulsePath, callbacks));
    key.addData(QJsonDocument::fromVariant(identity).toJson(QJsonDocument::Compact));
    const auto path =
        QDir(root_).filePath(QString::fromLatin1(key.result().toHex()) + QStringLiteral(".wav"));
    // Only atomic, complete writes are admitted. The cache directory is private to this process.
    if (QFileInfo(path).size() > 44)
        return {path, true};
    QSaveFile output(path);
    output.setDirectWriteFallback(false);
    if (!output.open(QIODevice::WriteOnly))
        throw std::runtime_error(output.errorString().toStdString());
    QtRenderByteSink sink(output);
    [[maybe_unused]] const auto rendered =
        echo::audio::OfflineWavRenderer::render(source.toStdString(), adjustment, sink, callbacks);
    if (sourceDigest(source, callbacks) != before)
        throw std::runtime_error("audio source changed while preparing the mix");
    if (callbacks.cancelled && callbacks.cancelled())
        throw echo::audio::OfflineRenderCancelled();
    if (!output.commit())
        throw std::runtime_error(output.errorString().toStdString());
    return {path, false};
}

void PreparedAssemblySourceCache::trim(quint64 maximumBytes) {
    const auto entries =
        QDir(root_).entryInfoList({QStringLiteral("*.wav")}, QDir::Files, QDir::Time);
    quint64 total = 0;
    for (const auto& file : entries)
        total += static_cast<quint64>(file.size());
    for (auto it = entries.crbegin(); it != entries.crend() && total > maximumBytes; ++it) {
        if (QFile::remove(it->absoluteFilePath()))
            total -= static_cast<quint64>(it->size());
    }
}
