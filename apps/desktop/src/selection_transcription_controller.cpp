#include "selection_transcription_controller.hpp"
#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"
#include <QFileInfo>
#include <QFutureWatcher>
#include <QtConcurrent/QtConcurrentRun>
#include <utility>
namespace {
struct Outcome {
    QString value;
    bool success = false;
};
} // namespace
SelectionTranscriptionController::SelectionTranscriptionController(
    QString catalog,
    QString cache,
    QObject* parent,
    TranscribeFunction transcribe
) :
    QObject(parent), transcribe_(std::move(transcribe)),
    catalog_(QFileInfo(catalog).absoluteFilePath()), cache_(QFileInfo(cache).absoluteFilePath()) {
    if (!transcribe_)
        transcribe_ = [](const QString& db,
                         const QString& root,
                         const QString& id,
                         qint64 start,
                         qint64 end,
                         const QString& endpoint) {
            const auto result = echo::desktop::transcribe_editor_selection(
                db.toStdString(),
                root.toStdString(),
                id.toStdString(),
                static_cast<std::uint64_t>(start),
                static_cast<std::uint64_t>(end),
                endpoint.toStdString()
            );
            return QString::fromUtf8(result.data(), static_cast<qsizetype>(result.size()));
        };
}
void SelectionTranscriptionController::request(
    const QString& id,
    qint64 start,
    qint64 end,
    const QString& endpoint,
    const QString& key
) {
    if (running_)
        return;
    discard();
    discarded_ = false;
    if (id.isEmpty() || start < 0 || end <= start || end - start > 300000) {
        error_ = tr("Select up to five minutes of original audio.");
        emit stateChanged();
        return;
    }
    running_ = true;
    key_ = key;
    const auto generation = generation_;
    emit stateChanged();
    auto* watcher = new QFutureWatcher<Outcome>(this);
    connect(watcher, &QFutureWatcher<Outcome>::finished, this, [this, watcher, generation] {
        const auto outcome = watcher->result();
        watcher->deleteLater();
        running_ = false;
        if (generation == generation_) {
            if (outcome.success)
                result_ = outcome.value;
            else
                error_ = tr(
                    "Transcription is unavailable. Check Infer Runtime and its local speech model."
                );
            emit resultChanged();
        }
        emit stateChanged();
    });
    watcher->setFuture(
        QtConcurrent::run([catalog = catalog_,
                           cache = cache_,
                           id,
                           start,
                           end,
                           endpoint,
                           transcribe = transcribe_] {
            Outcome outcome;
            try {
                outcome.value = transcribe(catalog, cache, id, start, end, endpoint);
                outcome.success = true;
            } catch (const std::exception&) {}
            return outcome;
        })
    );
}
void SelectionTranscriptionController::discard() {
    ++generation_;
    discarded_ = running_;
    result_.clear();
    key_.clear();
    error_.clear();
    emit resultChanged();
    emit stateChanged();
}
