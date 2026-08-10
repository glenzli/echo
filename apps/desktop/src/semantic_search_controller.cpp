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
    std::vector<std::pair<QString, double>> hits;
    bool succeeded = false;
};

} // namespace

SemanticSearchController::SemanticSearchController(
    QString catalogPath,
    QString runtimeEndpoint,
    QObject* parent
) :
    QObject(parent), catalog_path_(std::move(catalogPath)),
    runtime_endpoint_(std::move(runtimeEndpoint)) {}

void SemanticSearchController::request(const QString& query) {
    const QString normalized = query.simplified();
    if (normalized.isEmpty()) {
        clear();
        return;
    }
    const std::uint64_t generation = ++generation_;
    running_ = true;
    error_text_.clear();
    emit stateChanged();

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
            if (generation != generation_) {
                return;
            }
            running_ = false;
            results_.clear();
            results_query_ = normalized;
            if (outcome.succeeded) {
                for (const auto& [asset_id, score] : outcome.hits) {
                    results_.append(
                        QVariantMap{
                            {QStringLiteral("id"), asset_id},
                            {QStringLiteral("score"), score},
                        }
                    );
                }
                error_text_.clear();
            } else {
                error_text_ = QStringLiteral("semantic search unavailable");
            }
            emit resultsChanged();
            emit stateChanged();
        }
    );
    watcher->setFuture(QtConcurrent::run([catalog_path, runtime_endpoint, normalized] {
        SearchOutcome outcome;
        try {
            const auto hits = echo::desktop::semantic_search_catalog(
                catalog_path.toStdString(),
                runtime_endpoint.toStdString(),
                normalized.toStdString(),
                40
            );
            const std::size_t visible_count = std::min(
                hits.size(),
                std::clamp((hits.size() + 3) / 4, std::size_t{3}, std::size_t{12})
            );
            outcome.hits.reserve(visible_count);
            for (std::size_t index = 0; index < visible_count; ++index) {
                const auto& hit = hits[index];
                outcome.hits.emplace_back(
                    QString::fromUtf8(hit.asset_id.data(), hit.asset_id.size()),
                    hit.score
                );
            }
            outcome.succeeded = true;
        } catch (const rust::Error&) {
            // Literal search remains available. Do not log the private query.
        }
        return outcome;
    }));
}

void SemanticSearchController::clear() {
    ++generation_;
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
