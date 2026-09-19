#include "sound_assembly_controller.hpp"

#include "desktop_backend.hpp"
#include "playback_adjustment_projection.hpp"
#include "playback_controller.hpp"
#include "qt_render_byte_sink.hpp"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QHash>
#include <QMetaObject>
#include <QSaveFile>

#include <chrono>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "echo/audio/offline_assembly_wav_renderer.hpp"
#include "echo/audio/offline_wav_renderer.hpp"

namespace {

struct PreparedSourceJob {
    QString source_path;
    QString prepared_path;
    echo::audio::PlaybackAdjustment adjustment;
};

struct AssemblyJob {
    QString assembly_id;
    qint64 revision_id = 0;
    std::vector<PreparedSourceJob> sources;
    echo::audio::AssemblyMixPlan plan;
};

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

std::optional<echo::audio::AssemblyFadeCurve> assembly_fade_curve(const QVariant& value) {
    const QString stored = value.toString();
    if (stored == QStringLiteral("linear")) {
        return echo::audio::AssemblyFadeCurve::Linear;
    }
    if (stored == QStringLiteral("smooth")) {
        return echo::audio::AssemblyFadeCurve::Smooth;
    }
    if (stored == QStringLiteral("equal_power")) {
        return echo::audio::AssemblyFadeCurve::EqualPower;
    }
    bool numeric = false;
    const int legacy_value = value.toInt(&numeric);
    if (numeric && legacy_value >= 0 && legacy_value <= 2) {
        return static_cast<echo::audio::AssemblyFadeCurve>(legacy_value);
    }
    return std::nullopt;
}

std::optional<AssemblyJob>
assembly_job(const QVariantMap& revision, const QString& preparation_root, QString& error) {
    AssemblyJob job;
    job.assembly_id = revision.value(QStringLiteral("assemblyId")).toString();
    job.revision_id = revision.value(QStringLiteral("revisionId")).toLongLong();
    if (job.assembly_id.isEmpty() || job.revision_id <= 0) {
        error = QStringLiteral("assembly revision identity is invalid");
        return std::nullopt;
    }
    const QVariantList source_values = revision.value(QStringLiteral("clipSources")).toList();
    QHash<QString, QVariantMap> source_by_clip;
    for (const QVariant& item : source_values) {
        const QVariantMap source = item.toMap();
        source_by_clip.insert(source.value(QStringLiteral("clipId")).toString(), source);
    }

    QHash<QString, QString> prepared_by_key;
    const QVariantList track_values = revision.value(QStringLiteral("tracks")).toList();
    for (const QVariant& track_item : track_values) {
        const QVariantMap track_value = track_item.toMap();
        echo::audio::AssemblyTrackMix track;
        track.gain_centibels =
            static_cast<std::int16_t>(track_value.value(QStringLiteral("gainCentibels")).toInt());
        track.pan_percent =
            static_cast<std::int16_t>(track_value.value(QStringLiteral("panPercent")).toInt());
        track.muted = track_value.value(QStringLiteral("muted")).toBool();
        track.solo = track_value.value(QStringLiteral("solo")).toBool();
        for (const QVariant& clip_item : track_value.value(QStringLiteral("clips")).toList()) {
            const QVariantMap clip_value = clip_item.toMap();
            const QString clip_id = clip_value.value(QStringLiteral("id")).toString();
            if (!source_by_clip.contains(clip_id)) {
                error = QStringLiteral("assembly clip source cannot be resolved");
                return std::nullopt;
            }
            const QVariantMap source = source_by_clip.value(clip_id);
            const QString source_path = source.value(QStringLiteral("path")).toString();
            const qint64 adjustment_revision =
                source.value(QStringLiteral("adjustmentRevisionId")).toLongLong();
            const QString source_key =
                source_path + QChar(0x1f) + QString::number(adjustment_revision);
            QString prepared_path = prepared_by_key.value(source_key);
            if (prepared_path.isEmpty()) {
                const auto adjustment = PlaybackAdjustmentProjection::fromAssetMap(source);
                if (!adjustment.has_value()) {
                    error = QStringLiteral("assembly source adjustment is invalid");
                    return std::nullopt;
                }
                prepared_path =
                    QDir(preparation_root)
                        .filePath(QStringLiteral("source-%1.wav").arg(job.sources.size()));
                prepared_by_key.insert(source_key, prepared_path);
                job.sources.push_back({source_path, prepared_path, *adjustment});
            }
            echo::audio::AssemblyClipSource clip;
            clip.path = prepared_path.toStdString();
            clip.source_start_millis = static_cast<std::uint64_t>(
                clip_value.value(QStringLiteral("sourceStartMillis")).toULongLong()
            );
            clip.source_end_millis = static_cast<std::uint64_t>(
                clip_value.value(QStringLiteral("sourceEndMillis")).toULongLong()
            );
            clip.timeline_start_millis = static_cast<std::uint64_t>(
                clip_value.value(QStringLiteral("timelineStartMillis")).toULongLong()
            );
            clip.gain_centibels = static_cast<std::int16_t>(
                clip_value.value(QStringLiteral("gainCentibels")).toInt()
            );
            clip.pan_percent =
                static_cast<std::int16_t>(clip_value.value(QStringLiteral("panPercent")).toInt());
            clip.fade_in_millis = static_cast<std::uint64_t>(
                clip_value.value(QStringLiteral("fadeInMillis")).toULongLong()
            );
            clip.fade_out_millis = static_cast<std::uint64_t>(
                clip_value.value(QStringLiteral("fadeOutMillis")).toULongLong()
            );
            const auto fade_in_curve =
                assembly_fade_curve(clip_value.value(QStringLiteral("fadeInCurve")));
            const auto fade_out_curve =
                assembly_fade_curve(clip_value.value(QStringLiteral("fadeOutCurve")));
            if (!fade_in_curve.has_value() || !fade_out_curve.has_value()) {
                error = QStringLiteral("assembly clip fade curve is invalid");
                return std::nullopt;
            }
            clip.fade_in_curve = *fade_in_curve;
            clip.fade_out_curve = *fade_out_curve;
            clip.muted = clip_value.value(QStringLiteral("muted")).toBool();
            track.clips.push_back(std::move(clip));
        }
        job.plan.tracks.push_back(std::move(track));
    }
    const QVariantMap master = revision.value(QStringLiteral("master")).toMap();
    job.plan.master_gain_centibels =
        static_cast<std::int16_t>(master.value(QStringLiteral("gainCentibels")).toInt());
    job.plan.limiter_enabled = master.value(QStringLiteral("limiterEnabled"), true).toBool();
    job.plan.limiter_ceiling_centibels = static_cast<std::int16_t>(
        master.value(QStringLiteral("limiterCeilingCentibels"), -100).toInt()
    );
    job.plan.limiter_release_millis = static_cast<std::uint16_t>(
        master.value(QStringLiteral("limiterReleaseMillis"), 100).toInt()
    );
    if (job.sources.empty() || job.plan.tracks.empty()) {
        error = QStringLiteral("assembly has no renderable clips");
        return std::nullopt;
    }
    return job;
}

} // namespace

SoundAssemblyController::SoundAssemblyController(
    DesktopBackend& backend,
    PlaybackController& player,
    QObject* parent
) :
    QObject(parent), backend_(backend), player_(player),
    preview_directory_(QDir::temp().filePath(QStringLiteral("echo-assembly-XXXXXX"))) {
    preview_directory_.setAutoRemove(true);
}

SoundAssemblyController::~SoundAssemblyController() {
    stopWorker();
}

void SoundAssemblyController::preparePreview(const QVariantMap& revision) {
    if (!preview_directory_.isValid()) {
        reject(tr("Cannot create the private assembly preview directory."));
        return;
    }
    const std::uint64_t next_generation = generation_.load() + 1U;
    start(
        revision,
        QDir(preview_directory_.path())
            .filePath(QStringLiteral("preview-%1.wav").arg(next_generation)),
        true
    );
}

void SoundAssemblyController::exportAssembly(const QVariantMap& revision, const QUrl& destination) {
    if (!destination.isLocalFile()) {
        reject(tr("The assembly destination must be a local file."));
        return;
    }
    start(revision, normalized_destination(destination), false);
}

void SoundAssemblyController::saveToMemory(const QVariantMap& revision) {
    const auto destination =
        backend_.memoryOutputDestination(revision.value(QStringLiteral("id")).toString());
    if (destination.contains(QStringLiteral("error"))) {
        reject(destination.value(QStringLiteral("error")).toString());
        return;
    }
    start(revision, destination.value(QStringLiteral("path")).toString(), false, true);
}

void SoundAssemblyController::start(
    const QVariantMap& revision,
    const QString& destination,
    bool preview,
    bool preserveMemory
) {
    if (destination.isEmpty()) {
        reject(tr("The assembly destination is invalid."));
        return;
    }
    if (preview) {
        player_.stop();
        if (!preview_path_.isEmpty()) {
            QFile::remove(preview_path_);
            preview_path_.clear();
        }
        has_preview_ = false;
    }
    const QVariantList source_values = revision.value(QStringLiteral("clipSources")).toList();
    for (const QVariant& item : source_values) {
        if (same_file(item.toMap().value(QStringLiteral("path")).toString(), destination)) {
            reject(tr("The mixdown cannot replace an immutable Original."));
            return;
        }
    }
    stopWorker();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    const QString preparation_root =
        QDir(preview_directory_.path()).filePath(QStringLiteral("job-%1").arg(generation));
    if (!QDir().mkpath(preparation_root)) {
        reject(tr("Cannot create the private assembly preparation directory."));
        return;
    }
    QString projection_error;
    auto projected = assembly_job(revision, preparation_root, projection_error);
    if (!projected.has_value()) {
        reject(projection_error);
        return;
    }

    running_ = true;
    has_result_ = false;
    progress_ = 0.0;
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();
    worker_ = std::jthread([this,
                            generation,
                            destination,
                            preview,
                            preserveMemory,
                            preparation_root,
                            job = std::move(*projected)](std::stop_token stop_token) mutable {
        try {
            const double preparation_share = 0.6;
            auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
            for (std::size_t index = 0; index < job.sources.size(); ++index) {
                if (stop_token.stop_requested()) {
                    throw echo::audio::OfflineRenderCancelled();
                }
                const auto& source = job.sources[index];
                QSaveFile prepared(source.prepared_path);
                prepared.setDirectWriteFallback(false);
                if (!prepared.open(QIODevice::WriteOnly)) {
                    throw std::runtime_error(prepared.errorString().toStdString());
                }
                QtRenderByteSink sink(prepared);
                [[maybe_unused]] const auto prepared_result =
                    echo::audio::OfflineWavRenderer::render(
                        source.source_path.toStdString(),
                        source.adjustment,
                        sink,
                        {
                            .cancelled = [&stop_token] { return stop_token.stop_requested(); },
                            .progress =
                                [this,
                                 generation,
                                 index,
                                 count = job.sources.size(),
                                 &last_progress](double value) {
                                    const auto now = std::chrono::steady_clock::now();
                                    if (value < 1.0
                                        && now - last_progress < std::chrono::milliseconds(80)) {
                                        return;
                                    }
                                    last_progress = now;
                                    const double overall = 0.6
                                                           * (static_cast<double>(index) + value)
                                                           / static_cast<double>(count);
                                    QMetaObject::invokeMethod(
                                        this,
                                        [this, generation, overall] {
                                            if (generation_.load() == generation) {
                                                progress_ = overall;
                                                emit progressChanged();
                                            }
                                        },
                                        Qt::QueuedConnection
                                    );
                                },
                        }
                    );
                if (!prepared.commit()) {
                    throw std::runtime_error(prepared.errorString().toStdString());
                }
            }

            QSaveFile output(destination);
            output.setDirectWriteFallback(false);
            if (!output.open(QIODevice::WriteOnly)) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            QtRenderByteSink sink(output);
            const auto result = echo::audio::OfflineAssemblyWavRenderer::render(
                job.plan,
                sink,
                {
                    .cancelled = [&stop_token] { return stop_token.stop_requested(); },
                    .progress =
                        [this, generation, preparation_share, &last_progress](double value) {
                            const auto now = std::chrono::steady_clock::now();
                            if (value < 1.0
                                && now - last_progress < std::chrono::milliseconds(80)) {
                                return;
                            }
                            last_progress = now;
                            const double overall =
                                preparation_share + (1.0 - preparation_share) * value;
                            QMetaObject::invokeMethod(
                                this,
                                [this, generation, overall] {
                                    if (generation_.load() == generation) {
                                        progress_ = overall;
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
            if (!output.commit()) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            QDir(preparation_root).removeRecursively();
            QString publication_error;
            if (!preview) {
                publication_error = backend_.recordSoundAssemblyExport(
                    job.assembly_id,
                    job.revision_id,
                    destination,
                    result.sample_rate,
                    result.channel_count,
                    result.bit_depth,
                    result.frame_count,
                    result.size_bytes,
                    result.integrated_lufs,
                    result.true_peak_dbtp,
                    preserveMemory
                );
            }
            QMetaObject::invokeMethod(
                this,
                [this,
                 generation,
                 destination,
                 preview,
                 preserveMemory,
                 assemblyId = job.assembly_id,
                 result,
                 publication_error] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = !preview && publication_error.isEmpty();
                    has_preview_ = preview;
                    progress_ = 1.0;
                    integrated_lufs_ = result.integrated_lufs;
                    true_peak_dbtp_ = result.true_peak_dbtp;
                    error_text_ = publication_error;
                    if (preview) {
                        preview_path_ = destination;
                        player_.play(destination);
                    } else {
                        output_path_ = destination;
                    }
                    if (preserveMemory && publication_error.isEmpty()) {
                        backend_.refresh();
                        emit memorySaved(assemblyId);
                    }
                    emit progressChanged();
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        } catch (const echo::audio::OfflineRenderCancelled&) {
            QDir(preparation_root).removeRecursively();
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
        } catch (const std::exception& exception) {
            QDir(preparation_root).removeRecursively();
            const QString message = QString::fromUtf8(exception.what());
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

void SoundAssemblyController::cancel() {
    stopWorker();
    running_ = false;
    progress_ = 0.0;
    emit progressChanged();
    emit stateChanged();
}

bool SoundAssemblyController::running() const {
    return running_;
}
bool SoundAssemblyController::hasPreview() const {
    return has_preview_;
}
bool SoundAssemblyController::hasResult() const {
    return has_result_;
}
qreal SoundAssemblyController::progress() const {
    return progress_;
}
QString SoundAssemblyController::previewPath() const {
    return preview_path_;
}
QString SoundAssemblyController::outputPath() const {
    return output_path_;
}
qreal SoundAssemblyController::integratedLufs() const {
    return integrated_lufs_;
}
qreal SoundAssemblyController::truePeakDbtp() const {
    return true_peak_dbtp_;
}
QString SoundAssemblyController::errorText() const {
    return error_text_;
}

void SoundAssemblyController::stopWorker() {
    generation_.fetch_add(1);
    if (worker_.joinable()) {
        worker_.request_stop();
        worker_.join();
    }
}

void SoundAssemblyController::reject(const QString& message) {
    stopWorker();
    running_ = false;
    has_result_ = false;
    error_text_ = message;
    emit stateChanged();
}
