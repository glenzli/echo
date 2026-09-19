#include "assembly_waveform_controller.hpp"

#include <QFutureWatcher>
#include <QtConcurrent/QtConcurrentRun>

#include <algorithm>
#include <utility>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

namespace {
QVariantList sourceWaveform(const QString& catalog, const QString& cache, const QString& id) {
    try {
        // A worker-owned session keeps QObject and catalog projection work off
        // the GUI thread. This does not start inference or scanning workers.
        const auto session =
            echo::desktop::open_session(catalog.toStdString(), cache.toStdString());
        const auto artifact = session->session_waveform_artifact(id.toStdString());
        if (artifact.levels.empty())
            return {};
        const auto& level = artifact.levels[0];
        const std::size_t stride = std::max<std::size_t>(1, (level.mins.size() + 8191) / 8192);
        QVariantList mins, maxs;
        for (std::size_t start = 0; start < level.mins.size(); start += stride) {
            float minimum = 0, maximum = 0;
            for (std::size_t i = start; i < std::min(start + stride, level.mins.size()); ++i) {
                minimum = std::min(minimum, level.mins[i]);
                maximum = std::max(maximum, level.maxs[i]);
            }
            mins.append(minimum);
            maxs.append(maximum);
        }
        return {QVariantMap{
            {QStringLiteral("mins"), mins},
            {QStringLiteral("maxs"), maxs},
            {QStringLiteral("samplesPerBucket"),
             static_cast<int>(level.samples_per_bucket * stride)}
        }};
    } catch (const rust::Error&) {
        return {};
    }
}
} // namespace

AssemblyWaveformController::AssemblyWaveformController(
    QString catalogPath,
    QString cacheRoot,
    QObject* parent
) : QObject(parent), catalog_path_(std::move(catalogPath)), cache_root_(std::move(cacheRoot)) {
    pool_.setMaxThreadCount(1);
}

AssemblyWaveformController::~AssemblyWaveformController() {
    pool_.clear();
    pool_.waitForDone();
}

void AssemblyWaveformController::setSources(const QStringList& assetIds) {
    wanted_.clear();
    for (const auto& id : assetIds.mid(0, 256))
        if (!id.isEmpty())
            wanted_.insert(id);
    for (auto it = waveforms_.begin(); it != waveforms_.end();) {
        if (!wanted_.contains(it.key()))
            it = waveforms_.erase(it);
        else
            ++it;
    }
    emit waveformsChanged();
    requestNext();
}

void AssemblyWaveformController::requestNext() {
    if (!pending_.isEmpty())
        return;
    for (const auto& id : wanted_) {
        if (waveforms_.contains(id) || pending_.contains(id))
            continue;
        pending_.insert(id);
        auto* watcher = new QFutureWatcher<QVariantList>(this);
        connect(watcher, &QFutureWatcher<QVariantList>::finished, this, [this, watcher, id] {
            const auto levels = watcher->result();
            watcher->deleteLater();
            pending_.remove(id);
            if (wanted_.contains(id)) {
                waveforms_.insert(id, levels);
                emit waveformsChanged();
            }
            requestNext();
        });
        watcher->setFuture(
            QtConcurrent::run(&pool_, [catalog = catalog_path_, cache = cache_root_, id] {
                return sourceWaveform(catalog, cache, id);
            })
        );
        break;
    }
}

QVariantMap AssemblyWaveformController::waveforms() const {
    return waveforms_;
}
