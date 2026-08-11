#include "batch_export_controller.hpp"

#include "desktop_backend.hpp"
#include "playback_adjustment_projection.hpp"
#include "qt_render_byte_sink.hpp"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QMetaObject>
#include <QRegularExpression>
#include <QSaveFile>
#include <QSet>
#include <QStandardPaths>
#include <QUuid>

#include <algorithm>
#include <chrono>
#include <stdexcept>

#include "echo/audio/offline_flac_renderer.hpp"
#include "echo/audio/offline_wav_renderer.hpp"

namespace {

constexpr int kManifestVersion = 2;
constexpr int kMaximumBatchSize = 10'000;

QString manifest_path() {
    return QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation))
        .filePath(QStringLiteral("delivery-queue.json"));
}

bool save_manifest(const QVariantMap& manifest) {
    const QString path = manifest_path();
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        return false;
    }
    QSaveFile file(path);
    file.setDirectWriteFallback(false);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    const QByteArray payload = QJsonDocument::fromVariant(manifest).toJson(QJsonDocument::Compact);
    if (file.write(payload) != payload.size()) {
        file.cancelWriting();
        return false;
    }
    if (!file.commit())
        return false;
    return QFile::setPermissions(path, QFileDevice::ReadOwner | QFileDevice::WriteOwner);
}

QVariantMap load_manifest() {
    QFile file(manifest_path());
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        return {};
    }
    return document.toVariant().toMap();
}

QString extension_for(const QString& format) {
    return format == QStringLiteral("flac24") ? QStringLiteral("flac") : QStringLiteral("wav");
}

bool supported_format(const QString& format) {
    return format == QStringLiteral("wav_pcm16") || format == QStringLiteral("wav_pcm24")
           || format == QStringLiteral("flac24");
}

QString display_name(const QVariantMap& asset) {
    const QString caption = asset.value(QStringLiteral("soundCaption")).toString().trimmed();
    if (!caption.isEmpty())
        return caption;
    const QString title = asset.value(QStringLiteral("sourceTitle")).toString().trimmed();
    if (!title.isEmpty())
        return title;
    return QFileInfo(asset.value(QStringLiteral("path")).toString()).completeBaseName();
}

QString safe_stem(const QVariantMap& asset) {
    QString stem = display_name(asset);
    stem.replace(
        QRegularExpression(QStringLiteral("[\\x00-\\x1f/\\\\:*?\"<>|]+")),
        QStringLiteral("-")
    );
    stem = stem.simplified().left(80);
    while (stem.endsWith(u'.') || stem.endsWith(u' '))
        stem.chop(1);
    return stem.isEmpty() ? QStringLiteral("Sound") : stem;
}

QVariantMap snapshot_asset(const QVariantMap& asset) {
    static const QStringList keys{
        QStringLiteral("id"),
        QStringLiteral("path"),
        QStringLiteral("pathStatus"),
        QStringLiteral("soundCaption"),
        QStringLiteral("sourceTitle"),
        QStringLiteral("adjustmentRevision"),
        QStringLiteral("trimStartMillis"),
        QStringLiteral("trimEndMillis"),
        QStringLiteral("fadeInMillis"),
        QStringLiteral("fadeOutMillis"),
        QStringLiteral("fadeInCurve"),
        QStringLiteral("fadeOutCurve"),
        QStringLiteral("gainCentibels"),
        QStringLiteral("lowCutHertz"),
        QStringLiteral("noiseReductionEnabled"),
        QStringLiteral("noiseReductionCentibels"),
        QStringLiteral("noiseReductionSensitivityPercent"),
        QStringLiteral("noiseReductionSmoothingMillis"),
        QStringLiteral("deEsserEnabled"),
        QStringLiteral("deEsserFrequencyHertz"),
        QStringLiteral("deEsserThresholdCentibels"),
        QStringLiteral("deEsserReductionCentibels"),
        QStringLiteral("equalizerBands"),
        QStringLiteral("compressorEnabled"),
        QStringLiteral("compressorThresholdCentibels"),
        QStringLiteral("compressorRatioTenths"),
        QStringLiteral("compressorAttackMillis"),
        QStringLiteral("compressorReleaseMillis"),
        QStringLiteral("compressorMakeupCentibels"),
        QStringLiteral("reverbCharacter"),
        QStringLiteral("reverbEnabled"),
        QStringLiteral("reverbMixPercent"),
        QStringLiteral("reverbPreDelayMillis"),
        QStringLiteral("reverbDecayMillis"),
        QStringLiteral("reverbSizePercent"),
        QStringLiteral("reverbDampingPercent"),
        QStringLiteral("reverbLowCutHertz"),
        QStringLiteral("reverbHighCutHertz"),
        QStringLiteral("limiterEnabled"),
        QStringLiteral("limiterCeilingCentibels"),
        QStringLiteral("limiterReleaseMillis")
    };
    QVariantMap snapshot;
    for (const QString& key : keys)
        snapshot.insert(key, asset.value(key));
    return snapshot;
}

QString unique_output_path(
    const QString& directory,
    const QString& stem,
    const QString& extension,
    const QSet<QString>& reserved
) {
    const QDir output(directory);
    for (int suffix = 1; suffix < 100'000; ++suffix) {
        const QString name = suffix == 1
                                 ? QStringLiteral("%1.%2").arg(stem, extension)
                                 : QStringLiteral("%1-%2.%3").arg(stem).arg(suffix).arg(extension);
        const QString path = QFileInfo(output.filePath(name)).absoluteFilePath();
        if (!QFileInfo::exists(path) && !reserved.contains(path))
            return path;
    }
    throw std::runtime_error("cannot allocate a unique delivery name");
}

echo::audio::OfflineRenderResult render_file(
    const QString& source,
    const QString& temporaryPath,
    const QString& destinationPath,
    const QString& format,
    const echo::audio::PlaybackAdjustment& adjustment,
    const echo::audio::OfflineRenderCallbacks& callbacks
) {
    QFile::remove(temporaryPath);
    QFile output(temporaryPath);
    if (!output.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
        throw std::runtime_error(output.errorString().toStdString());
    }
    try {
        QtRenderByteSink sink(output);
        echo::audio::OfflineRenderResult result;
        if (format == QStringLiteral("flac24")) {
            result = echo::audio::OfflineFlacRenderer::render(
                source.toStdString(),
                adjustment,
                sink,
                callbacks
            );
        } else {
            const auto depth = format == QStringLiteral("wav_pcm16")
                                   ? echo::audio::WavPcmDepth::Pcm16
                                   : echo::audio::WavPcmDepth::Pcm24;
            result = echo::audio::OfflineWavRenderer::render(
                source.toStdString(),
                adjustment,
                sink,
                callbacks,
                depth
            );
        }
        if (!output.flush()) {
            throw std::runtime_error(output.errorString().toStdString());
        }
        output.close();
        if (QFileInfo::exists(destinationPath) || !QFile::rename(temporaryPath, destinationPath)) {
            throw std::runtime_error("cannot publish rendered file atomically");
        }
        return result;
    } catch (...) {
        output.close();
        QFile::remove(temporaryPath);
        throw;
    }
}

} // namespace

BatchExportController::BatchExportController(DesktopBackend& backend, QObject* parent) :
    QObject(parent), backend_(backend) {
    loadRecoveryManifest();
}

BatchExportController::~BatchExportController() {
    stopWorker(false);
}

void BatchExportController::start(
    const QVariantList& assets,
    const QUrl& destinationDirectory,
    const QString& format
) {
    if (running_)
        return;
    if (assets.isEmpty() || assets.size() > kMaximumBatchSize || !supported_format(format)) {
        reject(QStringLiteral("batch export request is invalid"));
        return;
    }
    if (!destinationDirectory.isLocalFile()) {
        reject(QStringLiteral("batch export destination must be a local directory"));
        return;
    }
    const QFileInfo directory(destinationDirectory.toLocalFile());
    if (!directory.exists() || !directory.isDir() || !directory.isWritable()) {
        reject(QStringLiteral("batch export destination is not writable"));
        return;
    }

    QVariantList items;
    for (const QVariant& value : assets) {
        const QVariantMap asset = value.toMap();
        if (asset.value(QStringLiteral("id")).toString().isEmpty()
            || asset.value(QStringLiteral("pathStatus")).toString() == QStringLiteral("missing")
            || !PlaybackAdjustmentProjection::fromAssetMap(asset).has_value()) {
            continue;
        }
        QVariantMap item;
        item.insert(QStringLiteral("state"), QStringLiteral("pending"));
        item.insert(QStringLiteral("asset"), snapshot_asset(asset));
        item.insert(QStringLiteral("name"), display_name(asset));
        items.append(item);
    }
    if (items.isEmpty()) {
        reject(QStringLiteral("batch export has no available sounds"));
        return;
    }
    QVariantMap manifest{
        {QStringLiteral("version"), kManifestVersion},
        {QStringLiteral("state"), QStringLiteral("active")},
        {QStringLiteral("format"), format},
        {QStringLiteral("directory"), directory.absoluteFilePath()},
        {QStringLiteral("items"), items},
    };
    if (!save_manifest(manifest)) {
        reject(QStringLiteral("batch export recovery state cannot be saved"));
        return;
    }
    startWorker(manifest);
}

void BatchExportController::resume() {
    if (!running_ && recoverable_ && !recovery_manifest_.isEmpty()) {
        startWorker(recovery_manifest_);
    }
}

void BatchExportController::cancel() {
    stopWorker(true);
}

void BatchExportController::dismiss() {
    if (running_)
        return;
    const QVariantList items = recovery_manifest_.value(QStringLiteral("items")).toList();
    for (const QVariant& value : items) {
        const QVariantMap item = value.toMap();
        const QString state = item.value(QStringLiteral("state")).toString();
        if (state == QStringLiteral("pending") || state == QStringLiteral("rendering")
            || state == QStringLiteral("publishing")) {
            QFile::remove(item.value(QStringLiteral("temporaryPath")).toString());
            QFile::remove(item.value(QStringLiteral("outputPath")).toString());
        }
    }
    recoverable_ = false;
    has_result_ = false;
    error_text_.clear();
    recovery_manifest_.clear();
    QFile::remove(manifest_path());
    emit stateChanged();
}

void BatchExportController::startWorker(QVariantMap manifest) {
    stopWorker(false);
    const QString format = manifest.value(QStringLiteral("format")).toString();
    const QString directory = manifest.value(QStringLiteral("directory")).toString();
    QVariantList items = manifest.value(QStringLiteral("items")).toList();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    user_cancelled_.store(false);
    running_ = true;
    recoverable_ = false;
    has_result_ = false;
    progress_ = 0.0;
    total_count_ = items.size();
    completed_count_ = 0;
    failed_count_ = 0;
    current_name_.clear();
    output_directory_ = directory;
    format_ = format;
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();

    worker_ = std::jthread([this, generation, manifest, items, directory, format](
                               std::stop_token stop
                           ) mutable {
        int completed = 0;
        int failed = 0;
        QSet<QString> reserved;
        for (const QVariant& value : items) {
            const QString output = value.toMap().value(QStringLiteral("outputPath")).toString();
            if (!output.isEmpty())
                reserved.insert(output);
        }
        auto persist = [&] {
            manifest.insert(QStringLiteral("items"), items);
            return save_manifest(manifest);
        };
        for (int index = 0; index < items.size(); ++index) {
            QVariantMap item = items[index].toMap();
            QString state = item.value(QStringLiteral("state")).toString();
            if (state == QStringLiteral("succeeded")) {
                ++completed;
                continue;
            }
            if (state == QStringLiteral("failed") || state == QStringLiteral("cancelled")) {
                ++failed;
                continue;
            }
            if (stop.stop_requested())
                break;

            const QVariantMap asset = item.value(QStringLiteral("asset")).toMap();
            const QString name = item.value(QStringLiteral("name")).toString();
            QMetaObject::invokeMethod(
                this,
                [this, generation, name] {
                    if (generation_.load() == generation) {
                        current_name_ = name;
                        emit stateChanged();
                    }
                },
                Qt::QueuedConnection
            );
            try {
                const auto adjustment = PlaybackAdjustmentProjection::fromAssetMap(asset);
                if (!adjustment.has_value()) {
                    throw std::runtime_error("saved adjustment is invalid");
                }
                QString output_path = item.value(QStringLiteral("outputPath")).toString();
                QString temporary_path = item.value(QStringLiteral("temporaryPath")).toString();
                if (state != QStringLiteral("publishing")) {
                    if (!temporary_path.isEmpty())
                        QFile::remove(temporary_path);
                    if (!output_path.isEmpty())
                        QFile::remove(output_path);
                    if (output_path.isEmpty()) {
                        output_path = unique_output_path(
                            directory,
                            safe_stem(asset),
                            extension_for(format),
                            reserved
                        );
                    }
                    reserved.insert(output_path);
                    temporary_path = output_path + QStringLiteral(".echo-partial-")
                                     + QUuid::createUuid().toString(QUuid::Id128);
                    item.insert(QStringLiteral("state"), QStringLiteral("rendering"));
                    item.insert(QStringLiteral("outputPath"), output_path);
                    item.insert(QStringLiteral("temporaryPath"), temporary_path);
                    items[index] = item;
                    if (!persist()) {
                        throw std::runtime_error("cannot save delivery recovery state");
                    }
                    auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
                    const auto result = render_file(
                        asset.value(QStringLiteral("path")).toString(),
                        temporary_path,
                        output_path,
                        format,
                        *adjustment,
                        {
                            .cancelled = [&stop] { return stop.stop_requested(); },
                            .progress =
                                [this, generation, index, total = items.size(), &last_progress](
                                    double value
                                ) {
                                    const auto now = std::chrono::steady_clock::now();
                                    if (value < 1.0
                                        && now - last_progress < std::chrono::milliseconds(80))
                                        return;
                                    last_progress = now;
                                    const double batch_progress =
                                        (static_cast<double>(index) + value)
                                        / static_cast<double>(total);
                                    QMetaObject::invokeMethod(
                                        this,
                                        [this, generation, batch_progress] {
                                            if (generation_.load() == generation) {
                                                progress_ = batch_progress;
                                                emit progressChanged();
                                            }
                                        },
                                        Qt::QueuedConnection
                                    );
                                },
                        }
                    );
                    if (stop.stop_requested()) {
                        QFile::remove(output_path);
                        throw echo::audio::OfflineRenderCancelled();
                    }
                    item.insert(QStringLiteral("state"), QStringLiteral("publishing"));
                    item.insert(
                        QStringLiteral("frameCount"),
                        static_cast<qulonglong>(result.frame_count)
                    );
                    item.insert(
                        QStringLiteral("sizeBytes"),
                        static_cast<qulonglong>(result.size_bytes)
                    );
                    item.insert(
                        QStringLiteral("sampleRate"),
                        static_cast<quint32>(result.sample_rate)
                    );
                    item.insert(
                        QStringLiteral("channelCount"),
                        static_cast<quint32>(result.channel_count)
                    );
                    item.insert(QStringLiteral("bitDepth"), static_cast<quint16>(result.bit_depth));
                    item.insert(QStringLiteral("integratedLufs"), result.integrated_lufs);
                    item.insert(QStringLiteral("truePeakDbtp"), result.true_peak_dbtp);
                    items[index] = item;
                    if (!persist()) {
                        QFile::remove(output_path);
                        throw std::runtime_error("cannot save rendered delivery state");
                    }
                }
                if (!QFileInfo::exists(output_path)) {
                    throw std::runtime_error("rendered delivery is missing before publication");
                }
                const QString publication_error = backend_.recordRenderExport(
                    asset.value(QStringLiteral("id")).toString(),
                    asset.value(QStringLiteral("adjustmentRevision")).toLongLong(),
                    output_path,
                    format,
                    item.value(QStringLiteral("sampleRate")).toUInt(),
                    item.value(QStringLiteral("channelCount")).toUInt(),
                    static_cast<quint16>(item.value(QStringLiteral("bitDepth")).toUInt()),
                    item.value(QStringLiteral("frameCount")).toULongLong(),
                    item.value(QStringLiteral("sizeBytes")).toULongLong(),
                    item.value(QStringLiteral("integratedLufs")).toFloat(),
                    item.value(QStringLiteral("truePeakDbtp")).toFloat()
                );
                if (!publication_error.isEmpty()) {
                    throw std::runtime_error(publication_error.toStdString());
                }
                item.insert(QStringLiteral("state"), QStringLiteral("succeeded"));
                item.remove(QStringLiteral("temporaryPath"));
                ++completed;
            } catch (const echo::audio::OfflineRenderCancelled&) {
                QFile::remove(item.value(QStringLiteral("temporaryPath")).toString());
                QFile::remove(item.value(QStringLiteral("outputPath")).toString());
                item.insert(QStringLiteral("state"), QStringLiteral("pending"));
            } catch (const std::exception&) {
                QFile::remove(item.value(QStringLiteral("temporaryPath")).toString());
                QFile::remove(item.value(QStringLiteral("outputPath")).toString());
                item.insert(QStringLiteral("state"), QStringLiteral("failed"));
                ++failed;
            }
            items[index] = item;
            persist();
            QMetaObject::invokeMethod(
                this,
                [this, generation, completed, failed] {
                    if (generation_.load() == generation) {
                        completed_count_ = completed;
                        failed_count_ = failed;
                        emit stateChanged();
                    }
                },
                Qt::QueuedConnection
            );
            if (stop.stop_requested())
                break;
        }

        const bool cancelled_by_user = stop.stop_requested() && user_cancelled_.load();
        if (stop.stop_requested()) {
            for (int index = 0; index < items.size(); ++index) {
                QVariantMap item = items[index].toMap();
                const QString state = item.value(QStringLiteral("state")).toString();
                if (state == QStringLiteral("rendering") || state == QStringLiteral("pending")
                    || (cancelled_by_user && state == QStringLiteral("publishing"))) {
                    if (cancelled_by_user) {
                        QFile::remove(item.value(QStringLiteral("temporaryPath")).toString());
                        QFile::remove(item.value(QStringLiteral("outputPath")).toString());
                    }
                    item.insert(
                        QStringLiteral("state"),
                        cancelled_by_user ? QStringLiteral("cancelled") : QStringLiteral("pending")
                    );
                    items[index] = item;
                }
            }
        }
        const bool unfinished =
            std::any_of(items.cbegin(), items.cend(), [](const QVariant& value) {
                const QString state = value.toMap().value(QStringLiteral("state")).toString();
                return state == QStringLiteral("pending") || state == QStringLiteral("rendering")
                       || state == QStringLiteral("publishing");
            });
        manifest.insert(
            QStringLiteral("state"),
            unfinished          ? QStringLiteral("paused")
            : cancelled_by_user ? QStringLiteral("cancelled")
                                : QStringLiteral("completed")
        );
        manifest.insert(QStringLiteral("items"), items);
        if (unfinished)
            save_manifest(manifest);
        else
            QFile::remove(manifest_path());

        QMetaObject::invokeMethod(
            this,
            [this, generation, manifest, completed, failed, unfinished] {
                if (generation_.load() != generation)
                    return;
                running_ = false;
                recoverable_ = unfinished;
                has_result_ = !unfinished;
                progress_ = unfinished ? progress_ : 1.0;
                completed_count_ = completed;
                failed_count_ = failed;
                current_name_.clear();
                recovery_manifest_ = unfinished ? manifest : QVariantMap{};
                emit progressChanged();
                emit stateChanged();
            },
            Qt::QueuedConnection
        );
    });
}

void BatchExportController::loadRecoveryManifest() {
    QVariantMap manifest = load_manifest();
    if (manifest.value(QStringLiteral("version")).toInt() != kManifestVersion
        || !supported_format(manifest.value(QStringLiteral("format")).toString())
        || manifest.value(QStringLiteral("items")).toList().isEmpty()) {
        QFile::remove(manifest_path());
        return;
    }
    QVariantList items = manifest.value(QStringLiteral("items")).toList();
    int completed = 0;
    int failed = 0;
    for (int index = 0; index < items.size(); ++index) {
        QVariantMap item = items[index].toMap();
        QString state = item.value(QStringLiteral("state")).toString();
        if (state == QStringLiteral("running") || state == QStringLiteral("rendering")) {
            state = QStringLiteral("pending");
            item.insert(QStringLiteral("state"), state);
            items[index] = item;
        }
        if (state == QStringLiteral("succeeded"))
            ++completed;
        if (state == QStringLiteral("failed"))
            ++failed;
    }
    manifest.insert(QStringLiteral("state"), QStringLiteral("paused"));
    manifest.insert(QStringLiteral("items"), items);
    save_manifest(manifest);
    recovery_manifest_ = manifest;
    recoverable_ = true;
    total_count_ = items.size();
    completed_count_ = completed;
    failed_count_ = failed;
    output_directory_ = manifest.value(QStringLiteral("directory")).toString();
    format_ = manifest.value(QStringLiteral("format")).toString();
    progress_ = total_count_ == 0 ? 0.0 : static_cast<qreal>(completed + failed) / total_count_;
}

void BatchExportController::stopWorker(bool userCancelled) {
    if (!worker_.joinable())
        return;
    user_cancelled_.store(userCancelled);
    worker_.request_stop();
    worker_.join();
}

void BatchExportController::reject(const QString& message) {
    error_text_ = message;
    has_result_ = false;
    emit stateChanged();
}

bool BatchExportController::running() const {
    return running_;
}
bool BatchExportController::recoverable() const {
    return recoverable_;
}
bool BatchExportController::hasResult() const {
    return has_result_;
}
qreal BatchExportController::progress() const {
    return progress_;
}
int BatchExportController::totalCount() const {
    return total_count_;
}
int BatchExportController::completedCount() const {
    return completed_count_;
}
int BatchExportController::failedCount() const {
    return failed_count_;
}
QString BatchExportController::currentName() const {
    return current_name_;
}
QString BatchExportController::outputDirectory() const {
    return output_directory_;
}
QString BatchExportController::format() const {
    return format_;
}
QString BatchExportController::errorText() const {
    return error_text_;
}
