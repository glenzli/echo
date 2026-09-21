#include "audio_stream_import_controller.hpp"
#include "echo/audio/audio_streams.hpp"
#include "echo/audio/decode.hpp"
#include "qt_render_byte_sink.hpp"
#include <QCryptographicHash>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFutureWatcher>
#include <QSaveFile>
#include <QTemporaryDir>
#include <QtConcurrent/QtConcurrentRun>
#include <algorithm>
#include <stdexcept>
namespace {
struct Outcome {
    QVariantList streams;
    QString error;
    QUrl prepared;
};
QString hashFile(const QString& path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        throw std::runtime_error("original container could not be read");
    QCryptographicHash hash(QCryptographicHash::Sha256);
    if (!hash.addData(&file))
        throw std::runtime_error("original identity could not be verified");
    return QString::fromLatin1(hash.result().toHex());
}
QUrl preserveStream(
    const QString& root,
    const QUrl& source,
    int index,
    const std::shared_ptr<std::atomic<bool>>& cancelled
) {
    const auto check = [&] {
        if (cancelled->load())
            throw echo::audio::OfflineRenderCancelled();
    };
    const auto containerRoot = QDir(root).filePath(QStringLiteral("media/container-audio"));
    if (!QDir().mkpath(containerRoot))
        throw std::runtime_error("container storage is unavailable");
    QTemporaryDir temporary(QDir(containerRoot).filePath(QStringLiteral(".intake-XXXXXX")));
    if (!temporary.isValid())
        throw std::runtime_error("container staging is unavailable");
    QFile input(source.toLocalFile());
    if (!input.open(QIODevice::ReadOnly))
        throw std::runtime_error("original container could not be opened");
    QFile staged(temporary.filePath(QStringLiteral("original")));
    if (!staged.open(QIODevice::WriteOnly))
        throw std::runtime_error("original container could not be preserved");
    QCryptographicHash hash(QCryptographicHash::Sha256);
    while (!input.atEnd()) {
        check();
        const auto bytes = input.read(1024 * 1024);
        if (bytes.isEmpty() && input.error() != QFile::NoError)
            throw std::runtime_error("original container read failed");
        hash.addData(bytes);
        if (staged.write(bytes) != bytes.size())
            throw std::runtime_error("original container copy failed");
    }
    staged.close();
    input.close();
    const auto digest = QString::fromLatin1(hash.result().toHex());
    check();
    if (hashFile(source.toLocalFile()) != digest)
        throw std::runtime_error("original changed during import");
    const auto directory = QDir(containerRoot).filePath(digest);
    if (!QDir().mkpath(directory))
        throw std::runtime_error("container storage could not be created");
    const auto original = QDir(directory).filePath(QStringLiteral("original"));
    if (QFileInfo::exists(original)) {
        if (hashFile(original) != digest)
            throw std::runtime_error("preserved original identity changed");
    } else if (!QFile::rename(staged.fileName(), original))
        throw std::runtime_error("original could not be published");
    check();
    const auto path = QDir(directory).filePath(QStringLiteral("track-%1.mka").arg(index));
    if (QFileInfo::exists(path)) {
        const auto metadata = echo::audio::probe(path.toStdString());
        bool hashMatches = false, streamMatches = false;
        for (const auto& entry : metadata.metadata) {
            const auto key = QString::fromStdString(entry.key).toLower();
            if (key == QStringLiteral("echo_source_sha256"))
                hashMatches = entry.value == digest.toStdString();
            if (key == QStringLiteral("echo_source_stream_index"))
                streamMatches = entry.value == std::to_string(index);
        }
        if (!hashMatches || !streamMatches)
            throw std::runtime_error("selected audio provenance changed");
        return QUrl::fromLocalFile(path);
    }
    QSaveFile output(path);
    output.setDirectWriteFallback(false);
    if (!output.open(QIODevice::WriteOnly))
        throw std::runtime_error("selected audio could not be created");
    QtRenderByteSink sink(output);
    echo::audio::copy_audio_stream(
        original.toStdString(),
        index,
        sink,
        digest.toStdString(),
        QFileInfo(source.toLocalFile()).fileName().left(255).toStdString(),
        {.cancelled = [cancelled] { return cancelled->load(); }}
    );
    check();
    if (!output.commit())
        throw std::runtime_error("selected audio could not be published");
    return QUrl::fromLocalFile(path);
}
} // namespace
AudioStreamImportController::AudioStreamImportController(QString root, QObject* parent) :
    QObject(parent), root_(std::move(root)) {}
AudioStreamImportController::~AudioStreamImportController() {
    if (cancelled_)
        cancelled_->store(true);
}
QString AudioStreamImportController::sourceName() const {
    return cursor_ < files_.size() ? QFileInfo(files_[cursor_].toLocalFile()).fileName()
                                   : QString{};
}
void AudioStreamImportController::cancel() {
    if (cancelled_)
        cancelled_->store(true);
    cancelled_.reset();
    files_.clear();
    ready_.clear();
    streams_.clear();
    busy_ = false;
    choosing_ = false;
    error_.clear();
    emit stateChanged();
}
void AudioStreamImportController::inspect(const QList<QUrl>& files) {
    if (busy_ || choosing_)
        return;
    cancel();
    if (files.isEmpty() || files.size() > 256) {
        error_ = tr("Choose between 1 and 256 local audio files.");
        emit stateChanged();
        return;
    }
    for (const auto& file : files)
        if (!file.isLocalFile()) {
            error_ = tr("Choose a local audio file.");
            emit stateChanged();
            return;
        }
    files_ = files;
    cursor_ = 0;
    cancelled_ = std::make_shared<std::atomic<bool>>(false);
    next();
}
void AudioStreamImportController::next() {
    if (cursor_ >= files_.size()) {
        busy_ = false;
        choosing_ = false;
        emit stateChanged();
        const auto completed = ready_;
        emit filesReady(completed);
        return;
    }
    busy_ = true;
    choosing_ = false;
    streams_.clear();
    error_.clear();
    emit stateChanged();
    const auto token = cancelled_;
    const auto file = files_[cursor_];
    auto* watcher = new QFutureWatcher<Outcome>(this);
    connect(watcher, &QFutureWatcher<Outcome>::finished, this, [this, watcher, token, file] {
        const auto result = watcher->result();
        watcher->deleteLater();
        if (token != cancelled_ || token->load())
            return;
        busy_ = false;
        if (!result.error.isEmpty()) {
            error_ = tr("This file could not be inspected: %1").arg(result.error);
            emit stateChanged();
            return;
        }
        if (result.streams.isEmpty()) {
            error_ = tr("This file has no audio track.");
            emit stateChanged();
            return;
        }
        if (result.streams.size() == 1) {
            ready_.append(file);
            ++cursor_;
            next();
            return;
        }
        streams_ = result.streams;
        choosing_ = true;
        emit stateChanged();
    });
    watcher->setFuture(QtConcurrent::run([file] {
        Outcome result;
        try {
            for (const auto& stream : echo::audio::audio_streams(file.toLocalFile().toStdString()))
                result.streams.append(
                    QVariantMap{
                        {"index", stream.index},
                        {"codec", QString::fromStdString(stream.codec)},
                        {"title", QString::fromStdString(stream.title)},
                        {"language", QString::fromStdString(stream.language)},
                        {"sampleRate", stream.sample_rate},
                        {"channels", stream.channels},
                        {"durationMillis", static_cast<qulonglong>(stream.duration_millis)}
                    }
                );
        } catch (const std::exception& error) {
            result.error = QString::fromUtf8(error.what());
        }
        return result;
    }));
}
void AudioStreamImportController::select(int index) {
    if (busy_ || !choosing_
        || !std::any_of(streams_.cbegin(), streams_.cend(), [index](const auto& value) {
               return value.toMap().value("index").toInt() == index;
           }))
        return;
    busy_ = true;
    choosing_ = false;
    emit stateChanged();
    const auto token = cancelled_;
    const auto file = files_[cursor_];
    const auto root = root_;
    auto* watcher = new QFutureWatcher<Outcome>(this);
    connect(watcher, &QFutureWatcher<Outcome>::finished, this, [this, watcher, token] {
        const auto result = watcher->result();
        watcher->deleteLater();
        if (token != cancelled_ || token->load())
            return;
        busy_ = false;
        if (!result.error.isEmpty()) {
            error_ = tr("The selected audio track could not be imported: %1").arg(result.error);
            choosing_ = true;
            emit stateChanged();
            return;
        }
        ready_.append(result.prepared);
        ++cursor_;
        next();
    });
    watcher->setFuture(QtConcurrent::run([root, file, index, token] {
        Outcome result;
        try {
            result.prepared = preserveStream(root, file, index, token);
        } catch (const std::exception& error) {
            result.error = QString::fromUtf8(error.what());
        }
        return result;
    }));
}
