#include "click_analysis_controller.hpp"
#include <QtConcurrent/QtConcurrentRun>
#include <algorithm>
#include <cmath>
#include <utility>

ClickAnalysisController::ClickAnalysisController(QObject* parent, Analyze analyze) :
    QObject(parent), analyze_(std::move(analyze)) {
    if (!analyze_)
        analyze_ = echo::audio::analyze_clicks;
}
ClickAnalysisController::~ClickAnalysisController() {
    cancellation_.request_stop();
    if (watcher_)
        watcher_->waitForFinished();
}
void ClickAnalysisController::cancel() {
    ++generation_;
    pending_.reset();
    cancellation_.request_stop();
    cancelling_ = watcher_ != nullptr;
    result_.clear();
    key_.clear();
    error_.clear();
    emit stateChanged();
}
void ClickAnalysisController::scan(
    const QString& path,
    qint64 start,
    qint64 end,
    int sensitivity,
    int maximumMicroseconds,
    const QString& identity
) {
    cancel();
    if (path.isEmpty() || identity.isEmpty() || start < 0 || end <= start || end - start > 300000
        || end > 14400000 || sensitivity < 0 || sensitivity > 100 || maximumMicroseconds < 50
        || maximumMicroseconds > 2000) {
        error_ = tr("Select up to five minutes of original audio to check for clicks.");
        emit stateChanged();
        return;
    }
    cancelling_ = false;
    pending_ = Request{path, start, end, sensitivity, maximumMicroseconds, identity, generation_};
    startPending();
    emit stateChanged();
}
void ClickAnalysisController::startPending() {
    if (watcher_ || !pending_)
        return;
    const auto request = *pending_;
    pending_.reset();
    cancellation_ = std::stop_source{};
    const auto token = cancellation_.get_token();
    watcher_ = new QFutureWatcher<Outcome>(this);
    connect(watcher_, &QFutureWatcher<Outcome>::finished, this, [this, request] {
        const auto outcome = watcher_->result();
        watcher_->deleteLater();
        watcher_ = nullptr;
        if (request.generation == generation_) {
            if (outcome.success) {
                QVariantList items;
                for (const auto& candidate : outcome.result.candidates) {
                    items.push_back(
                        QVariantMap{
                            {"startMillis", static_cast<qlonglong>(candidate.start_frame / 48)},
                            {"endMillis", static_cast<qlonglong>((candidate.end_frame + 47) / 48)},
                            {"channelMask", candidate.channel_mask},
                            {"differenceDb",
                             20.0 * std::log10(std::max(candidate.maximum_difference, 0.00001F))}
                        }
                    );
                }
                result_ = {
                    {"items", items},
                    {"total", static_cast<qulonglong>(outcome.result.total_candidates)},
                    {"startMillis", request.start},
                    {"endMillis", request.end}
                };
                key_ = request.key;
            } else
                error_ =
                    tr("Click analysis could not finish. Check that the original file is available "
                       "and unchanged.");
        }
        cancelling_ = false;
        startPending();
        emit stateChanged();
    });
    watcher_->setFuture(QtConcurrent::run([request, token, analyze = analyze_] {
        Outcome outcome;
        try {
            outcome.result = analyze(
                request.path.toStdString(),
                static_cast<std::uint64_t>(request.start),
                static_cast<std::uint64_t>(request.end),
                {.enabled = true,
                 .sensitivity_percent = static_cast<std::uint8_t>(request.sensitivity),
                 .maximum_click_microseconds = static_cast<std::uint16_t>(request.maximum),
                 .repair_percent = 100},
                token
            );
            outcome.success = !token.stop_requested();
        } catch (const std::exception&) {}
        return outcome;
    }));
}
