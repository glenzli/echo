#include "semantic_search_controller.hpp"

#include <QFutureWatcher>
#include <QVariantMap>
#include <QtConcurrent/QtConcurrentRun>

#include <algorithm>
#include <string>
#include <utility>
#include <vector>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

namespace {

struct SearchOutcome {
    QVariantList hits;
    bool succeeded = false;
};

} // namespace

SemanticSearchController::SemanticSearchController(
    QString catalogPath,
    QString runtimeEndpoint,
    QObject* parent,
    SearchFunction search
) :
    QObject(parent), catalog_path_(std::move(catalogPath)),
    runtime_endpoint_(std::move(runtimeEndpoint)) {
    search_ =
        search ? std::move(search)
               : SearchFunction(
                     [](const QString& catalog, const QString& endpoint, const QString& query) {
                         const auto hits = echo::desktop::semantic_search_catalog(
                             catalog.toStdString(),
                             endpoint.toStdString(),
                             query.toStdString(),
                             40
                         );
                         const std::size_t count = std::min(
                             hits.size(),
                             std::clamp((hits.size() + 3) / 4, std::size_t{3}, std::size_t{12})
                         );
                         QVariantList results;
                         for (std::size_t index = 0; index < count; ++index) {
                             const auto& hit = hits[index];
                             results.append(
                                 QVariantMap{
                                     {QStringLiteral("id"),
                                      QString::fromUtf8(hit.asset_id.data(), hit.asset_id.size())},
                                     {QStringLiteral("score"), hit.score}
                                 }
                             );
                         }
                         return results;
                     }
                 );
}

void SemanticSearchController::request(const QString& query) {
    const QString normalized = query.simplified();
    if (normalized.isEmpty()) {
        clear();
        return;
    }
    ++generation_;
    pending_query_ = normalized;
    running_ = true;
    error_text_.clear();
    emit stateChanged();

    startPending();
}

void SemanticSearchController::startPending() {
    if (worker_active_ || pending_query_.isEmpty())
        return;
    worker_active_ = true;
    const QString normalized = std::exchange(pending_query_, {});
    const std::uint64_t generation = generation_;
    const QString catalog_path = catalog_path_;
    const QString runtime_endpoint = runtime_endpoint_;
    auto* watcher = new QFutureWatcher<SearchOutcome>(this);
    connect(
        watcher,
        &QFutureWatcher<SearchOutcome>::finished,
        this,
        [this, watcher, generation, normalized] {
            const SearchOutcome outcome = watcher->result();
            watcher->deleteLater();
            worker_active_ = false;
            if (generation != generation_) {
                startPending();
                return;
            }
            running_ = false;
            results_.clear();
            results_query_ = normalized;
            if (outcome.succeeded) {
                results_ = outcome.hits;
                error_text_.clear();
            } else {
                error_text_ = QStringLiteral("semantic search unavailable");
            }
            emit resultsChanged();
            emit stateChanged();
            startPending();
        }
    );
    watcher->setFuture(
        QtConcurrent::run([catalog_path, runtime_endpoint, normalized, search = search_] {
            SearchOutcome outcome;
            try {
                outcome.hits = search(catalog_path, runtime_endpoint, normalized);
                outcome.succeeded = true;
            } catch (const std::exception&) {
                // Literal search remains available. Do not log the private query.
            }
            return outcome;
        })
    );
}

void SemanticSearchController::clear() {
    ++generation_;
    pending_query_.clear();
    const bool state_changed = running_ || !error_text_.isEmpty();
    const bool results_changed = !results_.isEmpty() || !results_query_.isEmpty();
    running_ = false;
    error_text_.clear();
    results_.clear();
    results_query_.clear();
    if (results_changed) {
        emit resultsChanged();
    }
    if (state_changed) {
        emit stateChanged();
    }
}

void SemanticSearchController::setRuntimeEndpoint(const QString& endpoint) {
    const QString normalized = endpoint.trimmed();
    if (normalized == runtime_endpoint_) {
        return;
    }
    clear();
    runtime_endpoint_ = normalized;
}

QVariantList SemanticSearchController::results() const {
    return results_;
}

QString SemanticSearchController::resultsQuery() const {
    return results_query_;
}

bool SemanticSearchController::running() const {
    return running_;
}

QString SemanticSearchController::errorText() const {
    return error_text_;
}
