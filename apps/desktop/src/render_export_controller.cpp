#include "render_export_controller.hpp"

#include "desktop_backend.hpp"
#include "playback_adjustment_projection.hpp"
#include "qt_render_byte_sink.hpp"

#include <QFileInfo>
#include <QMetaObject>
#include <QSaveFile>

#include <chrono>
#include <stdexcept>
#include <string>

#include "echo/audio/offline_wav_renderer.hpp"

namespace {

QString normalized_destination(const QUrl& destination) {
    QString path = destination.toLocalFile();
    if (!path.endsWith(QStringLiteral(".wav"), Qt::CaseInsensitive)) {
        path += QStringLiteral(".wav");
    }
    return QFileInfo(path).absoluteFilePath();
}

bool same_file(const QString& source, const QString& destination) {
    const QFileInfo source_info(source);
    const QFileInfo destination_info(destination);
    const QString source_canonical = source_info.canonicalFilePath();
    const QString destination_canonical = destination_info.canonicalFilePath();
    if (!source_canonical.isEmpty() && !destination_canonical.isEmpty()) {
        return source_canonical == destination_canonical;
    }
    return source_info.absoluteFilePath() == destination_info.absoluteFilePath();
}

} // namespace

RenderExportController::RenderExportController(DesktopBackend& backend, QObject* parent) :
    QObject(parent), backend_(backend) {}

RenderExportController::~RenderExportController() {
    stopWorker();
}

void RenderExportController::exportAdjusted(
    const QString& assetId,
    qint64 adjustmentRevisionId,
    const QString& sourcePath,
    const QUrl& destination,
    qint64 trimStartMillis,
    qint64 trimEndMillis,
    qint64 fadeInMillis,
    qint64 fadeOutMillis,
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    const QVariantMap& restoration,
    const QVariantMap& deHum,
    const QVariantMap& deClick,
    const QVariantMap& channelRepair,
    bool equalizerEnabled,
    const QVariantList& equalizerBands,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    const QVariantMap& reverb,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis,
    const QVariantList& effectChain,
    const QVariantList& editSegments,
    const QVariantList& effectMasks
) {
    exportAdjusted(
        assetId,
        adjustmentRevisionId,
        sourcePath,
        destination,
        trimStartMillis,
        trimEndMillis,
        fadeInMillis,
        fadeOutMillis,
        fadeInCurve,
        fadeOutCurve,
        gainCentibels,
        lowCutHertz,
        restoration,
        deHum,
        deClick,
        channelRepair,
        equalizerEnabled,
        equalizerBands,
        compressorEnabled,
        compressorThresholdCentibels,
        compressorRatioTenths,
        compressorAttackMillis,
        compressorReleaseMillis,
        compressorMakeupCentibels,
        reverb,
        limiterEnabled,
        limiterCeilingCentibels,
        limiterReleaseMillis,
        effectChain,
        editSegments,
        effectMasks,
        {}
    );
}

void RenderExportController::exportAdjusted(
    const QString& assetId,
    qint64 adjustmentRevisionId,
    const QString& sourcePath,
    const QUrl& destination,
    qint64 trimStartMillis,
    qint64 trimEndMillis,
    qint64 fadeInMillis,
    qint64 fadeOutMillis,
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    const QVariantMap& restoration,
    const QVariantMap& deHum,
    const QVariantMap& deClick,
    const QVariantMap& channelRepair,
    bool equalizerEnabled,
    const QVariantList& equalizerBands,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    const QVariantMap& reverb,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis,
    const QVariantList& effectChain,
    const QVariantList& editSegments,
    const QVariantList& effectMasks,
    const QVariantMap& creativeVfx,
    const QVariantMap& spectralRepair
) {
    if (assetId.isEmpty() || adjustmentRevisionId < 0 || sourcePath.isEmpty()) {
        reject(QStringLiteral("render source identity is invalid"));
        return;
    }
    if (!destination.isLocalFile()) {
        reject(QStringLiteral("render destination must be a local file"));
        return;
    }
    const QString output_path = normalized_destination(destination);
    if (output_path.isEmpty() || same_file(sourcePath, output_path)) {
        reject(QStringLiteral("render destination cannot replace the immutable original"));
        return;
    }
    const auto adjustment = PlaybackAdjustmentProjection::fromQml(
        trimStartMillis,
        trimEndMillis,
        fadeInMillis,
        fadeOutMillis,
        fadeInCurve,
        fadeOutCurve,
        gainCentibels,
        lowCutHertz,
        restoration,
        deHum,
        deClick,
        channelRepair,
        equalizerEnabled,
        equalizerBands,
        compressorEnabled,
        compressorThresholdCentibels,
        compressorRatioTenths,
        compressorAttackMillis,
        compressorReleaseMillis,
        compressorMakeupCentibels,
        reverb,
        limiterEnabled,
        limiterCeilingCentibels,
        limiterReleaseMillis,
        effectChain,
        editSegments,
        effectMasks,
        creativeVfx,
        spectralRepair
    );
    if (!adjustment.has_value()) {
        reject(QStringLiteral("render adjustment is outside the supported range"));
        return;
    }

    stopWorker();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    running_ = true;
    has_result_ = false;
    progress_ = 0.0;
    output_path_.clear();
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();

    const std::string source_path = sourcePath.toStdString();
    worker_ = std::jthread([this,
                            generation,
                            assetId,
                            adjustmentRevisionId,
                            source_path,
                            output_path,
                            adjustment = *adjustment](std::stop_token stop_token) {
        QSaveFile output(output_path);
        output.setDirectWriteFallback(false);
        try {
            if (!output.open(QIODevice::WriteOnly)) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            const auto source_disclosure = backend_.exportSourceDisclosure(assetId);
            QtRenderByteSink sink(output);
            auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
            const auto result = echo::audio::OfflineWavRenderer::render(
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
                },
                echo::audio::WavPcmDepth::Pcm24,
                source_disclosure.toStdString()
            );
            if (stop_token.stop_requested()) {
                throw echo::audio::OfflineRenderCancelled();
            }
            if (!output.commit()) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            const QString publication_error = backend_.recordRenderExport(
                assetId,
                adjustmentRevisionId,
                output_path,
                QStringLiteral("wav_pcm24"),
                result.sample_rate,
                result.channel_count,
                result.bit_depth,
                result.frame_count,
                result.size_bytes,
                result.integrated_lufs,
                result.true_peak_dbtp,
                source_disclosure
            );
            QMetaObject::invokeMethod(
                this,
                [this, generation, output_path, result, publication_error] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = publication_error.isEmpty();
                    progress_ = 1.0;
                    output_path_ = output_path;
                    integrated_lufs_ = result.integrated_lufs;
                    true_peak_dbtp_ = result.true_peak_dbtp;
                    error_text_ = publication_error;
                    emit progressChanged();
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        } catch (const echo::audio::OfflineRenderCancelled&) {
            output.cancelWriting();
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
            output.cancelWriting();
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

void RenderExportController::exportRenderedSpectralWorkingCopy(
    const QString& assetId,
    qint64 adjustmentRevisionId,
    qint64 workingCopyId,
    const QString& renderedSourcePath,
    const QUrl& destination
) {
    if (assetId.isEmpty() || adjustmentRevisionId < 0 || workingCopyId <= 0
        || renderedSourcePath.isEmpty()) {
        reject(QStringLiteral("rendered working-copy identity is invalid"));
        return;
    }
    if (!destination.isLocalFile()) {
        reject(QStringLiteral("render destination must be a local file"));
        return;
    }
    const QString output_path = normalized_destination(destination);
    if (output_path.isEmpty() || same_file(renderedSourcePath, output_path)) {
        reject(QStringLiteral("render destination cannot replace the rendered working copy"));
        return;
    }

    stopWorker();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    running_ = true;
    has_result_ = false;
    progress_ = 0.0;
    output_path_.clear();
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();

    const std::string source_path = renderedSourcePath.toStdString();
    worker_ = std::jthread([this,
                            generation,
                            assetId,
                            adjustmentRevisionId,
                            workingCopyId,
                            renderedSourcePath,
                            source_path,
                            output_path](std::stop_token stop_token) {
        QSaveFile output(output_path);
        output.setDirectWriteFallback(false);
        try {
            if (!output.open(QIODevice::WriteOnly)) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            const auto source_disclosure = backend_.exportSourceDisclosure(assetId);
            QtRenderByteSink sink(output);
            auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
            const echo::audio::PlaybackAdjustment no_downstream_adjustment;
            const auto result = echo::audio::OfflineWavRenderer::render(
                source_path,
                no_downstream_adjustment,
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
                },
                echo::audio::WavPcmDepth::Pcm24,
                source_disclosure.toStdString()
            );
            if (stop_token.stop_requested()) {
                throw echo::audio::OfflineRenderCancelled();
            }
            if (!output.commit()) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            const QString publication_error = backend_.recordRenderedSpectralWorkingCopyExport(
                assetId,
                adjustmentRevisionId,
                workingCopyId,
                renderedSourcePath,
                output_path,
                QStringLiteral("wav_pcm24"),
                result.sample_rate,
                result.channel_count,
                result.bit_depth,
                result.frame_count,
                result.size_bytes,
                result.integrated_lufs,
                result.true_peak_dbtp,
                source_disclosure
            );
            QMetaObject::invokeMethod(
                this,
                [this, generation, output_path, result, publication_error] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = publication_error.isEmpty();
                    progress_ = 1.0;
                    output_path_ = output_path;
                    integrated_lufs_ = result.integrated_lufs;
                    true_peak_dbtp_ = result.true_peak_dbtp;
                    error_text_ = publication_error;
                    emit progressChanged();
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        } catch (const echo::audio::OfflineRenderCancelled&) {
            output.cancelWriting();
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
            output.cancelWriting();
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

void RenderExportController::cancel() {
    if (worker_.joinable()) {
        worker_.request_stop();
    }
}

bool RenderExportController::running() const {
    return running_;
}

bool RenderExportController::hasResult() const {
    return has_result_;
}

qreal RenderExportController::progress() const {
    return progress_;
}

QString RenderExportController::outputPath() const {
    return output_path_;
}

qreal RenderExportController::integratedLufs() const {
    return integrated_lufs_;
}

qreal RenderExportController::truePeakDbtp() const {
    return true_peak_dbtp_;
}

QString RenderExportController::errorText() const {
    return error_text_;
}

void RenderExportController::stopWorker() {
    if (worker_.joinable()) {
        worker_.request_stop();
        worker_.join();
    }
}

void RenderExportController::reject(const QString& message) {
    stopWorker();
    generation_.fetch_add(1);
    running_ = false;
    has_result_ = false;
    progress_ = 0.0;
    output_path_.clear();
    error_text_ = message;
    emit progressChanged();
    emit stateChanged();
}
