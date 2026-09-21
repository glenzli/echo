#include "sound_assembly_controller.hpp"

#include "desktop_backend.hpp"
#include "playback_adjustment_projection.hpp"
#include "playback_controller.hpp"
#include "prepared_assembly_source_cache.hpp"
#include "qt_render_byte_sink.hpp"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QHash>
#include <QMetaObject>
#include <QSaveFile>

#include <chrono>
#include <cmath>
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
    QVariantMap identity;
};

struct AssemblyJob {
    QString assembly_id;
    qint64 revision_id = 0;
    std::vector<PreparedSourceJob> sources;
    echo::audio::AssemblyMixPlan plan;
};

// Only mix controls and display names can change under a prepared source plan.
QVariantMap mix_topology(const QVariantMap& revision) {
    QVariantList tracks;
    for (const auto& item : revision.value(QStringLiteral("tracks")).toList()) {
        auto track = item.toMap();
        for (const auto* key : {"gainCentibels", "panPercent", "muted", "solo", "name"})
            track.remove(QLatin1String(key));
        tracks.push_back(track);
    }
    auto master = revision.value(QStringLiteral("master")).toMap();
    master.remove(QStringLiteral("gainCentibels"));
    return {
        {QStringLiteral("id"), revision.value(QStringLiteral("id"))},
        {QStringLiteral("assemblyId"), revision.value(QStringLiteral("assemblyId"))},
        {QStringLiteral("tracks"), tracks},
        {QStringLiteral("master"), master},
        {QStringLiteral("sources"), revision.value(QStringLiteral("clipSources"))}
    };
}

bool mix_integer(const QVariant& value, int low, int high, std::int16_t& result) {
    bool ok = false;
    const double number = value.toDouble(&ok);
    if (!ok || !std::isfinite(number) || number < low || number > high
        || std::trunc(number) != number)
        return false;
    result = static_cast<std::int16_t>(number);
    return true;
}

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
                job.sources.push_back({source_path, prepared_path, *adjustment, source});
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
            const auto envelope = clip_value.value(QStringLiteral("gainEnvelope")).toMap();
            clip.gain_envelope_enabled = envelope.value(QStringLiteral("enabled")).toBool();
            for (const auto& value : envelope.value(QStringLiteral("points")).toList()) {
                const auto point = value.toMap();
                clip.gain_envelope.push_back(
                    {point.value(QStringLiteral("sourceMillis")).toULongLong(),
                     static_cast<std::int16_t>(
                         point.value(QStringLiteral("gainCentibels")).toInt()
                     )}
                );
            }
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
    player_.stop();
    stopWorker();
}

void SoundAssemblyController::preparePreview(const QVariantMap& revision) {
    prepareRangePreview(revision, 0, 0);
}

void SoundAssemblyController::prepareRangePreview(
    const QVariantMap& revision,
    qint64 startMillis,
    qint64 endMillis
) {
    if (startMillis < 0 || endMillis < 0 || (endMillis > 0 && endMillis <= startMillis)) {
        reject(tr("The preview range is invalid."));
        return;
    }
    if (!preview_directory_.isValid()) {
        reject(tr("Cannot create the private assembly preview directory."));
        return;
    }
    start(revision, preview_directory_.path(), true, false, startMillis, endMillis);
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
    bool preserveMemory,
    qint64 startMillis,
    qint64 endMillis
) {
    if (destination.isEmpty()) {
        reject(tr("The assembly destination is invalid."));
        return;
    }
    if (preview) {
        player_.stop();
        preview_plan_.reset();
        preview_revision_.clear();
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
    projected->plan.render_start_millis = static_cast<std::uint64_t>(startMillis);
    projected->plan.render_end_millis = static_cast<std::uint64_t>(endMillis);
    if (preview && endMillis > 0) {
        std::erase_if(projected->sources, [&](const PreparedSourceJob& source) {
            for (const auto& track : projected->plan.tracks)
                for (const auto& clip : track.clips)
                    if (clip.path == source.prepared_path.toStdString()
                        && clip.timeline_start_millis < static_cast<std::uint64_t>(endMillis)
                        && clip.timeline_start_millis + clip.source_end_millis
                                   - clip.source_start_millis
                               > static_cast<std::uint64_t>(startMillis))
                        return false;
            return true;
        });
    }
    QSet<QString> pinned_sources;
    if (preview_plan_)
        for (const auto& track : preview_plan_->tracks)
            for (const auto& clip : track.clips)
                pinned_sources.insert(QString::fromStdString(clip.path));
    worker_ = std::jthread([this,
                            revision,
                            generation,
                            destination,
                            preview,
                            preserveMemory,
                            preparation_root,
                            pinned_sources,
                            job = std::move(*projected)](std::stop_token stop_token) mutable {
        try {
            const double preparation_share = preview ? 1.0 : 0.6;
            PreparedAssemblySourceCache source_cache(
                QDir(preview_directory_.path()).filePath(QStringLiteral("sources"))
            );
            int reused_sources = 0;
            auto last_progress = std::chrono::steady_clock::now() - std::chrono::seconds(1);
            for (std::size_t index = 0; index < job.sources.size(); ++index) {
                if (stop_token.stop_requested()) {
                    throw echo::audio::OfflineRenderCancelled();
                }
                const auto& source = job.sources[index];
                const auto prepared_result = source_cache.prepare(
                    source.source_path,
                    source.identity,
                    source.adjustment,
                    {
                        .cancelled = [&stop_token] { return stop_token.stop_requested(); },
                        .progress =
                            [this, generation, index, count = job.sources.size(), &last_progress](
                                double value
                            ) {
                                const auto now = std::chrono::steady_clock::now();
                                if (value < 1.0
                                    && now - last_progress < std::chrono::milliseconds(80)) {
                                    return;
                                }
                                last_progress = now;
                                const double overall = 0.6 * (static_cast<double>(index) + value)
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
                if (prepared_result.reused)
                    ++reused_sources;
                for (auto& track : job.plan.tracks)
                    for (auto& clip : track.clips)
                        if (clip.path == source.prepared_path.toStdString())
                            clip.path = prepared_result.path.toStdString();
            }

            if (preview) {
                if (stop_token.stop_requested())
                    throw echo::audio::OfflineRenderCancelled();
                for (const auto& track : job.plan.tracks)
                    for (const auto& clip : track.clips)
                        pinned_sources.insert(QString::fromStdString(clip.path));
                source_cache.trim(4ULL * 1024ULL * 1024ULL * 1024ULL, pinned_sources);
                QDir(preparation_root).removeRecursively();
                QMetaObject::invokeMethod(
                    this,
                    [this,
                     generation,
                     revision,
                     plan = std::move(job.plan),
                     reused_sources]() mutable {
                        if (generation_.load() != generation)
                            return;
                        preview_plan_ = std::move(plan);
                        preview_revision_ = revision;
                        has_preview_ = true;
                        running_ = false;
                        reused_source_count_ = reused_sources;
                        progress_ = 1.0;
                        // The UI owns admission to playback and its initial seek.
                        emit progressChanged();
                        emit stateChanged();
                    },
                    Qt::QueuedConnection
                );
                return;
            }

            const auto source_disclosure =
                preview ? QString{}
                        : backend_.exportSourceDisclosure(job.assembly_id, job.revision_id);
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
                },
                source_disclosure.toStdString()
            );
            if (stop_token.stop_requested()) {
                throw echo::audio::OfflineRenderCancelled();
            }
            if (!output.commit()) {
                throw std::runtime_error(output.errorString().toStdString());
            }
            QDir(preparation_root).removeRecursively();
            source_cache.trim(4ULL * 1024ULL * 1024ULL * 1024ULL, pinned_sources);
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
                    preserveMemory,
                    source_disclosure
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
                 reused_sources,
                 publication_error] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    reused_source_count_ = reused_sources;
                    has_result_ = !preview && publication_error.isEmpty();
                    progress_ = 1.0;
                    integrated_lufs_ = result.integrated_lufs;
                    true_peak_dbtp_ = result.true_peak_dbtp;
                    error_text_ = publication_error;
                    output_path_ = destination;
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
bool SoundAssemblyController::playPreview(qint64 startMillis) {
    if (running_ || !has_preview_ || !preview_plan_ || startMillis < 0)
        return false;
    if (!player_.playAssembly(*preview_plan_)) {
        error_text_ = player_.errorText();
        emit stateChanged();
        return false;
    }
    if (startMillis > 0)
        player_.seek(startMillis);
    return true;
}
bool SoundAssemblyController::updatePreviewMix(const QVariantMap& revision, bool updatePlayback) {
    if (running_ || !has_preview_ || !preview_plan_
        || mix_topology(revision) != mix_topology(preview_revision_))
        return false;
    auto controls = echo::audio::assembly_mix_controls(*preview_plan_);
    const auto tracks = revision.value(QStringLiteral("tracks")).toList();
    if (static_cast<std::size_t>(tracks.size()) != controls.track_count)
        return false;
    for (qsizetype i = 0; i < tracks.size(); ++i) {
        const auto track = tracks[i].toMap();
        auto& control = controls.tracks[static_cast<std::size_t>(i)];
        if (!mix_integer(
                track.value(QStringLiteral("gainCentibels")),
                -2400,
                1200,
                control.gain_centibels
            )
            || !mix_integer(
                track.value(QStringLiteral("panPercent")),
                -100,
                100,
                control.pan_percent
            ))
            return false;
        control.muted = track.value(QStringLiteral("muted")).toBool();
        control.solo = track.value(QStringLiteral("solo")).toBool();
    }
    if (!mix_integer(
            revision.value(QStringLiteral("master")).toMap().value(QStringLiteral("gainCentibels")),
            -2400,
            1200,
            controls.master_gain_centibels
        ))
        return false;
    if (updatePlayback && !player_.updateAssemblyMix(controls))
        return false;
    for (std::size_t i = 0; i < controls.track_count; ++i) {
        auto& track = preview_plan_->tracks[i];
        const auto& control = controls.tracks[i];
        track.gain_centibels = control.gain_centibels;
        track.pan_percent = control.pan_percent;
        track.muted = control.muted;
        track.solo = control.solo;
    }
    preview_plan_->master_gain_centibels = controls.master_gain_centibels;
    preview_revision_ = revision;
    return true;
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
