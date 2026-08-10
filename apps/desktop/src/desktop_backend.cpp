#include "desktop_backend.hpp"

#include <QDebug>
#include <QDir>
#include <QFileInfo>
#include <QString>

DesktopBackend::DesktopBackend(rust::Box<echo::desktop::LibrarySession> session, QObject* parent) :
    QObject(parent), session_(std::move(session)) {}

void DesktopBackend::refresh() {
    emit assetsChanged();
}

QVariantList DesktopBackend::listAssets() const {
    QVariantList list;
    const auto assets = session_->session_list_assets();
    for (const auto& asset : assets) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), QString::fromUtf8(asset.id.data(), asset.id.size()));
        entry.insert(
            QStringLiteral("path"),
            QString::fromUtf8(asset.path.data(), asset.path.size())
        );
        entry.insert(
            QStringLiteral("codec"),
            QString::fromUtf8(asset.codec.data(), asset.codec.size())
        );
        entry.insert(
            QStringLiteral("durationMillis"),
            static_cast<qlonglong>(asset.duration_millis)
        );
        entry.insert(
            QStringLiteral("importedAtMillis"),
            static_cast<qlonglong>(asset.imported_at_millis)
        );
        entry.insert(
            QStringLiteral("recordedAtMillis"),
            static_cast<qlonglong>(asset.recorded_at_millis)
        );
        entry.insert(QStringLiteral("maxLevel"), static_cast<int>(asset.max_level));
        entry.insert(
            QStringLiteral("pathStatus"),
            QString::fromUtf8(asset.path_status.data(), asset.path_status.size())
        );
        entry.insert(
            QStringLiteral("soundCaption"),
            QString::fromUtf8(asset.sound_caption.data(), asset.sound_caption.size())
        );
        entry.insert(
            QStringLiteral("summary"),
            QString::fromUtf8(asset.summary.data(), asset.summary.size())
        );
        entry.insert(
            QStringLiteral("eventType"),
            QString::fromUtf8(asset.event_type.data(), asset.event_type.size())
        );
        entry.insert(
            QStringLiteral("mood"),
            QString::fromUtf8(asset.mood.data(), asset.mood.size())
        );
        entry.insert(
            QStringLiteral("textPreview"),
            QString::fromUtf8(asset.text_preview.data(), asset.text_preview.size())
        );
        entry.insert(QStringLiteral("liked"), asset.liked);
        entry.insert(QStringLiteral("rating"), static_cast<int>(asset.rating));
        entry.insert(
            QStringLiteral("adjustmentRevision"),
            static_cast<qlonglong>(asset.adjustment_revision)
        );
        entry.insert(
            QStringLiteral("trimStartMillis"),
            static_cast<qlonglong>(asset.trim_start_millis)
        );
        entry.insert(
            QStringLiteral("trimEndMillis"),
            static_cast<qlonglong>(asset.trim_end_millis)
        );
        entry.insert(QStringLiteral("fadeInMillis"), static_cast<qlonglong>(asset.fade_in_millis));
        entry.insert(
            QStringLiteral("fadeOutMillis"),
            static_cast<qlonglong>(asset.fade_out_millis)
        );
        entry.insert(QStringLiteral("fadeInCurve"), static_cast<int>(asset.fade_in_curve));
        entry.insert(QStringLiteral("fadeOutCurve"), static_cast<int>(asset.fade_out_curve));
        entry.insert(QStringLiteral("gainCentibels"), static_cast<int>(asset.gain_centibels));
        entry.insert(QStringLiteral("lowCutHertz"), static_cast<int>(asset.low_cut_hertz));
        entry.insert(
            QStringLiteral("eqLowGainCentibels"),
            static_cast<int>(asset.eq_low_gain_centibels)
        );
        entry.insert(
            QStringLiteral("eqMidGainCentibels"),
            static_cast<int>(asset.eq_mid_gain_centibels)
        );
        entry.insert(
            QStringLiteral("eqHighGainCentibels"),
            static_cast<int>(asset.eq_high_gain_centibels)
        );
        entry.insert(QStringLiteral("compressorEnabled"), asset.compressor_enabled);
        entry.insert(
            QStringLiteral("compressorThresholdCentibels"),
            static_cast<int>(asset.compressor_threshold_centibels)
        );
        entry.insert(
            QStringLiteral("compressorRatioTenths"),
            static_cast<int>(asset.compressor_ratio_tenths)
        );
        entry.insert(
            QStringLiteral("compressorAttackMillis"),
            static_cast<int>(asset.compressor_attack_millis)
        );
        entry.insert(
            QStringLiteral("compressorReleaseMillis"),
            static_cast<int>(asset.compressor_release_millis)
        );
        entry.insert(
            QStringLiteral("compressorMakeupCentibels"),
            static_cast<int>(asset.compressor_makeup_centibels)
        );
        entry.insert(QStringLiteral("limiterEnabled"), asset.limiter_enabled);
        entry.insert(
            QStringLiteral("limiterCeilingCentibels"),
            static_cast<int>(asset.limiter_ceiling_centibels)
        );
        entry.insert(
            QStringLiteral("limiterReleaseMillis"),
            static_cast<int>(asset.limiter_release_millis)
        );
        entry.insert(
            QStringLiteral("containerFormat"),
            QString::fromUtf8(asset.container_format.data(), asset.container_format.size())
        );
        entry.insert(QStringLiteral("sampleRate"), static_cast<int>(asset.sample_rate));
        entry.insert(QStringLiteral("channelCount"), static_cast<int>(asset.channel_count));
        entry.insert(
            QStringLiteral("sourceTitle"),
            QString::fromUtf8(asset.source_title.data(), asset.source_title.size())
        );
        entry.insert(
            QStringLiteral("sourceLocation"),
            QString::fromUtf8(asset.source_location.data(), asset.source_location.size())
        );
        entry.insert(
            QStringLiteral("sourceCreatedAt"),
            QString::fromUtf8(asset.source_created_at.data(), asset.source_created_at.size())
        );
        QVariantList keywords;
        for (const auto& keyword : asset.keywords) {
            keywords.append(QString::fromUtf8(keyword.data(), keyword.size()));
        }
        entry.insert(QStringLiteral("keywords"), keywords);
        list.append(entry);
    }
    return list;
}

QVariantList DesktopBackend::listKeywordFacets() const {
    QVariantList list;
    try {
        const auto facets = session_->session_keyword_facets();
        for (const auto& facet : facets) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("key"),
                QString::fromUtf8(facet.key.data(), facet.key.size())
            );
            entry.insert(
                QStringLiteral("label"),
                QString::fromUtf8(facet.label.data(), facet.label.size())
            );
            entry.insert(QStringLiteral("count"), static_cast<qulonglong>(facet.count));
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list contextual keyword facets: %s", error.what());
    }
    return list;
}

QVariantList DesktopBackend::listSmartAlbums() const {
    QVariantList list;
    try {
        const auto albums = session_->session_smart_albums();
        for (const auto& album : albums) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("key"),
                QString::fromUtf8(album.key.data(), album.key.size())
            );
            entry.insert(
                QStringLiteral("label"),
                QString::fromUtf8(album.label.data(), album.label.size())
            );
            entry.insert(
                QStringLiteral("facet"),
                QString::fromUtf8(album.facet.data(), album.facet.size())
            );
            entry.insert(
                QStringLiteral("evidence"),
                QString::fromUtf8(album.evidence.data(), album.evidence.size())
            );
            entry.insert(QStringLiteral("count"), static_cast<qulonglong>(album.count));
            QVariantList memberIds;
            for (const auto& memberId : album.member_asset_ids) {
                memberIds.append(QString::fromUtf8(memberId.data(), memberId.size()));
            }
            entry.insert(QStringLiteral("memberIds"), memberIds);
            list.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list smart album candidates: %s", error.what());
    }
    return list;
}

bool DesktopBackend::setAssetAffinity(const QString& id, bool liked, int rating) {
    if (rating < 0 || rating > 5) {
        qWarning("asset rating is outside zero to five");
        return false;
    }
    try {
        session_->session_set_asset_affinity(
            id.toStdString(),
            liked,
            static_cast<std::uint8_t>(rating)
        );
        emit assetsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot update affinity for %s: %s", qPrintable(id), error.what());
        return false;
    }
}

bool DesktopBackend::setAssetAdjustment(
    const QString& id,
    qlonglong trimStartMillis,
    qlonglong trimEndMillis,
    qlonglong fadeInMillis,
    qlonglong fadeOutMillis,
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    int eqLowGainCentibels,
    int eqMidGainCentibels,
    int eqHighGainCentibels,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis
) {
    if (trimStartMillis < 0 || trimEndMillis < 0 || fadeInMillis < 0 || fadeOutMillis < 0
        || fadeInCurve < 0 || fadeInCurve > 2 || fadeOutCurve < 0 || fadeOutCurve > 2
        || gainCentibels < -2400 || gainCentibels > 1200
        || (lowCutHertz != 0 && (lowCutHertz < 20 || lowCutHertz > 240))
        || eqLowGainCentibels < -1200 || eqLowGainCentibels > 1200 || eqMidGainCentibels < -1200
        || eqMidGainCentibels > 1200 || eqHighGainCentibels < -1200 || eqHighGainCentibels > 1200
        || compressorThresholdCentibels < -6000 || compressorThresholdCentibels > 0
        || compressorRatioTenths < 10 || compressorRatioTenths > 200 || compressorAttackMillis < 1
        || compressorAttackMillis > 200 || compressorReleaseMillis < 20
        || compressorReleaseMillis > 2000 || compressorMakeupCentibels < 0
        || compressorMakeupCentibels > 2400 || limiterCeilingCentibels < -600
        || limiterCeilingCentibels > 0 || limiterReleaseMillis < 20
        || limiterReleaseMillis > 1000) {
        qWarning("sound adjustment is outside the supported range");
        return false;
    }
    try {
        echo::desktop::AssetAdjustmentWire adjustment;
        adjustment.trim_start_millis = static_cast<std::uint64_t>(trimStartMillis);
        adjustment.trim_end_millis = static_cast<std::uint64_t>(trimEndMillis);
        adjustment.fade_in_millis = static_cast<std::uint64_t>(fadeInMillis);
        adjustment.fade_out_millis = static_cast<std::uint64_t>(fadeOutMillis);
        adjustment.fade_in_curve = static_cast<std::uint8_t>(fadeInCurve);
        adjustment.fade_out_curve = static_cast<std::uint8_t>(fadeOutCurve);
        adjustment.gain_centibels = static_cast<std::int16_t>(gainCentibels);
        adjustment.low_cut_hertz = static_cast<std::uint16_t>(lowCutHertz);
        adjustment.eq_low_gain_centibels = static_cast<std::int16_t>(eqLowGainCentibels);
        adjustment.eq_mid_gain_centibels = static_cast<std::int16_t>(eqMidGainCentibels);
        adjustment.eq_high_gain_centibels = static_cast<std::int16_t>(eqHighGainCentibels);
        adjustment.compressor_enabled = compressorEnabled;
        adjustment.compressor_threshold_centibels =
            static_cast<std::int16_t>(compressorThresholdCentibels);
        adjustment.compressor_ratio_tenths = static_cast<std::uint16_t>(compressorRatioTenths);
        adjustment.compressor_attack_millis = static_cast<std::uint16_t>(compressorAttackMillis);
        adjustment.compressor_release_millis = static_cast<std::uint16_t>(compressorReleaseMillis);
        adjustment.compressor_makeup_centibels =
            static_cast<std::int16_t>(compressorMakeupCentibels);
        adjustment.limiter_enabled = limiterEnabled;
        adjustment.limiter_ceiling_centibels = static_cast<std::int16_t>(limiterCeilingCentibels);
        adjustment.limiter_release_millis = static_cast<std::uint16_t>(limiterReleaseMillis);
        session_->session_set_asset_adjustment(id.toStdString(), adjustment);
        emit assetsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot update adjustment for %s: %s", qPrintable(id), error.what());
        return false;
    }
}

QVariantList DesktopBackend::waveformForAsset(const QString& id) const {
    QVariantList levels;
    const auto artifact = session_->session_waveform_artifact(id.toStdString());
    for (const auto& level : artifact.levels) {
        QVariantList mins;
        QVariantList maxs;
        for (const float sample : level.mins) {
            mins.append(static_cast<double>(sample));
        }
        for (const float sample : level.maxs) {
            maxs.append(static_cast<double>(sample));
        }
        QVariantMap entry;
        entry.insert(
            QStringLiteral("samplesPerBucket"),
            static_cast<int>(level.samples_per_bucket)
        );
        entry.insert(QStringLiteral("mins"), mins);
        entry.insert(QStringLiteral("maxs"), maxs);
        levels.append(entry);
    }
    return levels;
}

QVariantList DesktopBackend::transcriptsForAsset(const QString& id) const {
    QVariantList transcripts;
    rust::Vec<echo::desktop::TranscriptWire> wires;
    try {
        wires = session_->session_transcripts(id.toStdString());
    } catch (const rust::Error& error) {
        qWarning("transcript query failed for %s: %s", qPrintable(id), error.what());
        return transcripts;
    }
    qInfo("transcripts for %s: %zu record(s)", qPrintable(id), wires.size());
    for (const auto& wire : wires) {
        QVariantList segments;
        for (const auto& segment : wire.segments) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("text"),
                QString::fromUtf8(segment.text.data(), segment.text.size())
            );
            entry.insert(QStringLiteral("start"), segment.start);
            entry.insert(QStringLiteral("end"), segment.end);
            segments.append(entry);
        }
        QVariantMap record;
        record.insert(
            QStringLiteral("model"),
            QString::fromUtf8(wire.model.data(), wire.model.size())
        );
        record.insert(
            QStringLiteral("modelVersion"),
            QString::fromUtf8(wire.model_version.data(), wire.model_version.size())
        );
        record.insert(
            QStringLiteral("language"),
            QString::fromUtf8(wire.language.data(), wire.language.size())
        );
        record.insert(
            QStringLiteral("text"),
            QString::fromUtf8(wire.text.data(), wire.text.size())
        );
        record.insert(QStringLiteral("segments"), segments);
        transcripts.append(record);
    }
    return transcripts;
}

void DesktopBackend::startWorkers(const QString& runtimeEndpoint) {
    try {
        session_->session_start_workers(runtimeEndpoint.toStdString());
    } catch (const rust::Error& error) {
        qWarning("cannot start background workers: %s", error.what());
    }
}

QVariantMap DesktopBackend::analysisStatusForAsset(const QString& id) const {
    QVariantMap status;
    try {
        const auto wire = session_->session_analysis_status(id.toStdString());
        status.insert(
            QStringLiteral("stage"),
            QString::fromUtf8(wire.stage.data(), wire.stage.size())
        );
        status.insert(
            QStringLiteral("state"),
            QString::fromUtf8(wire.state.data(), wire.state.size())
        );
        status.insert(
            QStringLiteral("errorCode"),
            QString::fromUtf8(wire.error_code.data(), wire.error_code.size())
        );
        status.insert(
            QStringLiteral("runtimeJobId"),
            QString::fromUtf8(wire.runtime_job_id.data(), wire.runtime_job_id.size())
        );
        status.insert(
            QStringLiteral("contractVersion"),
            QString::fromUtf8(wire.contract_version.data(), wire.contract_version.size())
        );
    } catch (const rust::Error& error) {
        qWarning("cannot read analysis status for %s: %s", qPrintable(id), error.what());
    }
    return status;
}

bool DesktopBackend::retryAnalysis(const QString& id) {
    try {
        session_->session_retry_analysis(id.toStdString());
        emit jobsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot retry analysis for %s: %s", qPrintable(id), error.what());
        return false;
    }
}

void DesktopBackend::queueScans() {
    try {
        const auto queued = session_->session_queue_scans();
        qInfo("queued %llu background scan(s)", queued);
        emit jobsChanged();
    } catch (const rust::Error& error) {
        qWarning("cannot queue scans: %s", error.what());
    }
}

QVariantMap DesktopBackend::jobStats() const {
    QVariantMap stats;
    try {
        const auto wire = session_->session_job_stats();
        stats.insert(QStringLiteral("pending"), static_cast<qlonglong>(wire.pending));
        stats.insert(QStringLiteral("running"), static_cast<qlonglong>(wire.running));
        stats.insert(QStringLiteral("done"), static_cast<qlonglong>(wire.done));
        stats.insert(QStringLiteral("failed"), static_cast<qlonglong>(wire.failed));
    } catch (const rust::Error& error) {
        qWarning("cannot read job stats: %s", error.what());
    }
    return stats;
}

QVariantList DesktopBackend::listRoots() const {
    QVariantList roots;
    try {
        const auto wires = session_->session_list_roots();
        for (const auto& wire : wires) {
            QVariantMap entry;
            entry.insert(QStringLiteral("id"), static_cast<qlonglong>(wire.id));
            entry.insert(
                QStringLiteral("root"),
                QString::fromUtf8(wire.root.data(), wire.root.size())
            );
            entry.insert(QStringLiteral("enabled"), wire.enabled);
            roots.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("cannot list scan roots: %s", error.what());
    }
    return roots;
}

bool DesktopBackend::addRoot(const QUrl& folder) {
    if (!folder.isLocalFile()) {
        qWarning("cannot add non-local scan root %s", qPrintable(folder.toString()));
        return false;
    }
    const QString path = QDir::cleanPath(folder.toLocalFile());
    const QFileInfo info(path);
    if (!info.exists() || !info.isDir()) {
        qWarning("cannot add missing scan root %s", qPrintable(path));
        return false;
    }
    try {
        session_->session_add_root(path.toStdString());
        emit jobsChanged();
        return true;
    } catch (const rust::Error& error) {
        qWarning("cannot add scan root %s: %s", qPrintable(path), error.what());
        return false;
    }
}

void DesktopBackend::removeRoot(qlonglong id) {
    try {
        session_->session_remove_root(static_cast<std::int64_t>(id));
        emit jobsChanged();
    } catch (const rust::Error& error) {
        qWarning("cannot remove scan root: %s", error.what());
    }
}

QVariantList DesktopBackend::search(const QString& query) const {
    QVariantList results;
    if (query.trimmed().isEmpty()) {
        return results;
    }
    try {
        const auto wires = session_->session_search(query.toStdString(), 20);
        for (const auto& wire : wires) {
            QVariantMap entry;
            entry.insert(
                QStringLiteral("id"),
                QString::fromUtf8(wire.asset_id.data(), wire.asset_id.size())
            );
            entry.insert(
                QStringLiteral("path"),
                QString::fromUtf8(wire.path.data(), wire.path.size())
            );
            entry.insert(
                QStringLiteral("codec"),
                QString::fromUtf8(wire.codec.data(), wire.codec.size())
            );
            entry.insert(
                QStringLiteral("snippet"),
                QString::fromUtf8(wire.snippet.data(), wire.snippet.size())
            );
            entry.insert(QStringLiteral("startMillis"), static_cast<qlonglong>(wire.start_millis));
            results.append(entry);
        }
    } catch (const rust::Error& error) {
        qWarning("search failed: %s", error.what());
    }
    return results;
}

quint64 DesktopBackend::assetCount() const {
    return session_->session_asset_count();
}

QString DesktopBackend::catalogPath() const {
    const rust::String path = session_->session_catalog_path();
    return QString::fromUtf8(path.data(), path.size());
}

QString DesktopBackend::cacheRoot() const {
    const rust::String path = session_->session_cache_root();
    return QString::fromUtf8(path.data(), path.size());
}
