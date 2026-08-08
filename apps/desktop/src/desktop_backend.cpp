#include "desktop_backend.hpp"

#include <QDebug>
#include <QString>

DesktopBackend::DesktopBackend(rust::Box<echo::desktop::LibrarySession> session, QObject* parent) :
    QObject(parent), session_(std::move(session)) {}

DesktopBackend::~DesktopBackend() {
    if (analysis_thread_.joinable()) {
        analysis_thread_.join();
    }
}

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
        entry.insert(QStringLiteral("maxLevel"), static_cast<int>(asset.max_level));
        entry.insert(
            QStringLiteral("pathStatus"),
            QString::fromUtf8(asset.path_status.data(), asset.path_status.size())
        );
        list.append(entry);
    }
    return list;
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

void DesktopBackend::transcribeAsset(
    const QString& id,
    const QString& modelRoot,
    const QString& python,
    const QString& workerScript
) {
    if (transcribing_) {
        return;
    }
    transcribing_ = true;
    emit transcriptionStateChanged();

    const rust::String catalog_rust = session_->session_catalog_path();
    const std::string catalog(catalog_rust.data(), catalog_rust.size());
    const std::string asset = id.toStdString();
    const std::string root = modelRoot.toStdString();
    const std::string interpreter = python.toStdString();
    const std::string worker = workerScript.toStdString();

    analysis_thread_ = std::thread([this, catalog, asset, root, interpreter, worker] {
        QString message;
        bool ok = false;
        try {
            const auto segments =
                echo::desktop::transcribe_asset(catalog, asset, root, interpreter, worker);
            ok = true;
            message = QStringLiteral("%1 segments").arg(segments);
        } catch (const rust::Error& error) {
            message = QString::fromUtf8(error.what());
        }
        const QString asset_id = QString::fromStdString(asset);
        QMetaObject::invokeMethod(
            this,
            [this, asset_id, ok, message] {
                transcribing_ = false;
                emit transcriptionStateChanged();
                emit transcriptionFinished(asset_id, ok, message);
            },
            Qt::QueuedConnection
        );
    });
    analysis_thread_.detach();
}

bool DesktopBackend::transcribing() const {
    return transcribing_;
}

void DesktopBackend::startWorkers(
    const QString& modelRoot,
    const QString& python,
    const QString& workerScript,
    const QString& ollamaEndpoint,
    const QString& ollamaModel
) {
    try {
        session_->session_start_workers(
            modelRoot.toStdString(),
            python.toStdString(),
            workerScript.toStdString(),
            ollamaEndpoint.toStdString(),
            ollamaModel.toStdString()
        );
    } catch (const rust::Error& error) {
        qWarning("cannot start background workers: %s", error.what());
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

bool DesktopBackend::addRoot(const QString& path) {
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
