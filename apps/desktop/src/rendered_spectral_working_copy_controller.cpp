#include "rendered_spectral_working_copy_controller.hpp"

#include "desktop_backend.hpp"
#include "playback_adjustment_projection.hpp"
#include "qt_render_byte_sink.hpp"

#include <QDir>
#include <QMetaObject>
#include <QTemporaryFile>

#include <chrono>
#include <stdexcept>
#include <string>

#include "echo/audio/offline_wav_renderer.hpp"

RenderedSpectralWorkingCopyController::RenderedSpectralWorkingCopyController(
    DesktopBackend& backend,
    QObject* parent
) : QObject(parent), backend_(backend) {}

RenderedSpectralWorkingCopyController::~RenderedSpectralWorkingCopyController() {
    stopWorker();
}

void RenderedSpectralWorkingCopyController::createFromSavedAsset(const QVariantMap& asset) {
    const QString asset_id = asset.value(QStringLiteral("id")).toString();
    const QString source_path = asset.value(QStringLiteral("path")).toString();
    const qint64 adjustment_revision =
        asset.value(QStringLiteral("adjustmentRevision")).toLongLong();
    if (asset_id.isEmpty() || source_path.isEmpty() || adjustment_revision < 0
        || asset.value(QStringLiteral("pathStatus")).toString() == QStringLiteral("missing")) {
        reject(QStringLiteral("saved render source is unavailable"));
        return;
    }
    const auto adjustment = PlaybackAdjustmentProjection::fromAssetMap(asset);
    if (!adjustment.has_value()) {
        reject(QStringLiteral("saved render adjustment is outside the supported range"));
        return;
    }

    stopWorker();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    running_ = true;
    has_result_ = false;
    progress_ = 0.0;
    working_copy_id_ = 0;
    cache_path_.clear();
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();

    worker_ = std::jthread([this,
                            generation,
                            asset_id,
                            adjustment_revision,
                            source_path = source_path.toStdString(),
                            adjustment = *adjustment](std::stop_token stop_token) {
        QTemporaryFile rendered(
            QDir::temp().filePath(QStringLiteral("echo-spectral-working-copy-XXXXXX.wav"))
        );
        rendered.setAutoRemove(true);
        try {
            if (!rendered.open()) {
                throw std::runtime_error(rendered.errorString().toStdString());
            }
            const QString temporary_path = rendered.fileName();
            QtRenderByteSink sink(rendered);
            auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
            (void)echo::audio::OfflineWavRenderer::render(
                source_path,
                adjustment,
                sink,
                {
                    .cancelled = [&stop_token] { return stop_token.stop_requested(); },
                    .progress =
                        [this, generation, &last_progress](double value) {
                            const auto now = std::chrono::steady_clock::now();
                            if (value < 1.0
                                && now - last_progress < std::chrono::milliseconds(80)) {
                                return;
                            }
                            last_progress = now;
                            QMetaObject::invokeMethod(
                                this,
                                [this, generation, value] {
                                    if (generation_.load() == generation) {
                                        progress_ = value;
                                        emit progressChanged();
                                    }
                                },
                                Qt::QueuedConnection
                            );
                        },
                }
            );
            if (stop_token.stop_requested()) {
                throw echo::audio::OfflineRenderCancelled();
            }
            if (!rendered.flush()) {
                throw std::runtime_error(rendered.errorString().toStdString());
            }
            rendered.close();
            const QVariantMap published = backend_.createRenderedSpectralWorkingCopy(
                asset_id,
                adjustment_revision,
                temporary_path
            );
            const QString publication_error = published.value(QStringLiteral("error")).toString();
            if (!publication_error.isEmpty()) {
                throw std::runtime_error(publication_error.toStdString());
            }
            const qint64 working_copy_id = published.value(QStringLiteral("id")).toLongLong();
            const QString cache_path = published.value(QStringLiteral("cachePath")).toString();
            if (working_copy_id <= 0 || cache_path.isEmpty()) {
                throw std::runtime_error(
                    "rendered working-copy publication did not return cache identity"
                );
            }
            const quint32 operation_count =
                published.value(QStringLiteral("operationCount")).toUInt();
            QMetaObject::invokeMethod(
                this,
                [this, generation, working_copy_id, cache_path, operation_count] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = true;
                    progress_ = 1.0;
                    working_copy_id_ = working_copy_id;
                    cache_path_ = cache_path;
                    operation_count_ = operation_count;
                    error_text_.clear();
                    emit progressChanged();
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        } catch (const echo::audio::OfflineRenderCancelled&) {
            QMetaObject::invokeMethod(
                this,
                [this, generation] {
                    if (generation_.load() == generation) {
                        running_ = false;
                        has_result_ = false;
                        progress_ = 0.0;
                        emit progressChanged();
                        emit stateChanged();
                    }
                },
                Qt::QueuedConnection
            );
        } catch (const std::exception& error) {
            const QString message = QString::fromUtf8(error.what());
            QMetaObject::invokeMethod(
                this,
                [this, generation, message] {
                    if (generation_.load() == generation) {
                        running_ = false;
                        has_result_ = false;
                        error_text_ = message;
                        emit stateChanged();
                    }
                },
                Qt::QueuedConnection
            );
        }
    });
}

void RenderedSpectralWorkingCopyController::eraseRegion(
    const QString& assetId,
    qint64 workingCopyId,
    const QString& cachePath,
    qint64 startMillis,
    qint64 endMillis,
    int lowHertz,
    int highHertz
) {
    if (assetId.isEmpty() || workingCopyId <= 0 || cachePath.isEmpty() || startMillis < 0
        || endMillis <= startMillis || lowHertz < 20 || highHertz <= lowHertz
        || highHertz > 24'000) {
        reject(QStringLiteral("spectral erase selection is outside the supported range"));
        return;
    }

    stopWorker();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    running_ = true;
    progress_ = 0.0;
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();

    worker_ = std::jthread([this,
                            generation,
                            asset_id = assetId.toStdString(),
                            working_copy_id = workingCopyId,
                            cache_path = cachePath.toStdString(),
                            start_millis = static_cast<std::uint64_t>(startMillis),
                            end_millis = static_cast<std::uint64_t>(endMillis),
                            low_hertz = static_cast<std::uint16_t>(lowHertz),
                            high_hertz =
                                static_cast<std::uint16_t>(highHertz)](std::stop_token stop_token) {
        QTemporaryFile rendered(
            QDir::temp().filePath(QStringLiteral("echo-spectral-erase-XXXXXX.wav"))
        );
        rendered.setAutoRemove(true);
        try {
            if (!rendered.open()) {
                throw std::runtime_error(rendered.errorString().toStdString());
            }
            echo::audio::PlaybackAdjustment erase_adjustment;
            erase_adjustment.spectral_repair.push_back({
                .start_millis = start_millis,
                .end_millis = end_millis,
                .low_hertz = low_hertz,
                .high_hertz = high_hertz,
                .attenuation_centibels = 9'600,
                .time_feather_millis = 12,
                .frequency_feather_hertz = 48,
            });
            QtRenderByteSink sink(rendered);
            auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
            (void)echo::audio::OfflineWavRenderer::render(
                cache_path,
                erase_adjustment,
                sink,
                {
                    .cancelled = [&stop_token] { return stop_token.stop_requested(); },
                    .progress =
                        [this, generation, &last_progress](double value) {
                            const auto now = std::chrono::steady_clock::now();
                            if (value < 1.0
                                && now - last_progress < std::chrono::milliseconds(80)) {
                                return;
                            }
                            last_progress = now;
                            QMetaObject::invokeMethod(
                                this,
                                [this, generation, value] {
                                    if (generation_.load() == generation) {
                                        progress_ = value;
                                        emit progressChanged();
                                    }
                                },
                                Qt::QueuedConnection
                            );
                        },
                }
            );
            if (stop_token.stop_requested()) {
                throw echo::audio::OfflineRenderCancelled();
            }
            if (!rendered.flush()) {
                throw std::runtime_error(rendered.errorString().toStdString());
            }
            rendered.close();
            const QVariantMap published = backend_.commitRenderedSpectralErase(
                QString::fromStdString(asset_id),
                working_copy_id,
                rendered.fileName(),
                start_millis,
                end_millis,
                low_hertz,
                high_hertz,
                9'600,
                12,
                48
            );
            const QString publication_error = published.value(QStringLiteral("error")).toString();
            if (!publication_error.isEmpty()) {
                throw std::runtime_error(publication_error.toStdString());
            }
            const QString next_cache_path = published.value(QStringLiteral("cachePath")).toString();
            if (next_cache_path.isEmpty()) {
                throw std::runtime_error(
                    "spectral erase publication did not return cache identity"
                );
            }
            const quint32 operation_count =
                published.value(QStringLiteral("operationCount")).toUInt();
            QMetaObject::invokeMethod(
                this,
                [this, generation, next_cache_path, operation_count] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = true;
                    progress_ = 1.0;
                    cache_path_ = next_cache_path;
                    operation_count_ = operation_count;
                    error_text_.clear();
                    emit progressChanged();
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        } catch (const echo::audio::OfflineRenderCancelled&) {
            QMetaObject::invokeMethod(
                this,
                [this, generation] {
                    if (generation_.load() == generation) {
                        running_ = false;
                        progress_ = 0.0;
                        emit progressChanged();
                        emit stateChanged();
                    }
                },
                Qt::QueuedConnection
            );
        } catch (const std::exception& error) {
            const QString message = QString::fromUtf8(error.what());
            QMetaObject::invokeMethod(
                this,
                [this, generation, message] {
                    if (generation_.load() == generation) {
                        running_ = false;
                        error_text_ = message;
                        emit stateChanged();
                    }
                },
                Qt::QueuedConnection
            );
        }
    });
}

void RenderedSpectralWorkingCopyController::cancel() {
    if (worker_.joinable()) {
        worker_.request_stop();
    }
}

bool RenderedSpectralWorkingCopyController::running() const {
    return running_;
}

bool RenderedSpectralWorkingCopyController::hasResult() const {
    return has_result_;
}

qreal RenderedSpectralWorkingCopyController::progress() const {
    return progress_;
}

qint64 RenderedSpectralWorkingCopyController::workingCopyId() const {
    return working_copy_id_;
}

QString RenderedSpectralWorkingCopyController::cachePath() const {
    return cache_path_;
}

quint32 RenderedSpectralWorkingCopyController::operationCount() const {
    return operation_count_;
}

QString RenderedSpectralWorkingCopyController::errorText() const {
    return error_text_;
}

void RenderedSpectralWorkingCopyController::stopWorker() {
    if (worker_.joinable()) {
        worker_.request_stop();
        worker_.join();
    }
}

void RenderedSpectralWorkingCopyController::reject(const QString& message) {
    stopWorker();
    running_ = false;
    has_result_ = false;
    progress_ = 0.0;
    working_copy_id_ = 0;
    cache_path_.clear();
    operation_count_ = 0;
    error_text_ = message;
    emit progressChanged();
    emit stateChanged();
}
