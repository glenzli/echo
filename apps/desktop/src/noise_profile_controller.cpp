#include "noise_profile_controller.hpp"
#include "noise_profile_projection.hpp"

#include <QtConcurrent/QtConcurrentRun>

NoiseProfileController::NoiseProfileController(QObject* parent) : QObject(parent) {}
NoiseProfileController::~NoiseProfileController() {
    cancellation_.request_stop();
    if (watcher_)
        watcher_->waitForFinished();
}

void NoiseProfileController::capture(
    const QString& path,
    const QString& identity,
    qint64 startMillis,
    qint64 endMillis
) {
    cancel();
    if (path.isEmpty() || identity.isEmpty() || startMillis < 0 || endMillis <= startMillis
        || endMillis - startMillis < 100 || endMillis - startMillis > 30'000
        || endMillis > 14'400'000) {
        error_ = 1;
        emit stateChanged();
        return;
    }
    pending_ = Request{path, identity, startMillis, endMillis, generation_};
    running_ = true;
    emit stateChanged();
    startPending();
}

void NoiseProfileController::startPending() {
    if (watcher_ || !pending_)
        return;
    const auto request = *pending_;
    pending_.reset();
    cancellation_ = std::stop_source{};
    const auto token = cancellation_.get_token();
    watcher_ = new QFutureWatcher<QVariantMap>(this);
    connect(watcher_, &QFutureWatcher<QVariantMap>::finished, this, [this, request] {
        const auto result = watcher_->result();
        watcher_->deleteLater();
        watcher_ = nullptr;
        if (request.generation == generation_) {
            running_ = false;
            error_ = result.isEmpty() ? 2 : 0;
            emit stateChanged();
            if (!result.isEmpty())
                emit profileReady(request.identity, result);
        }
        startPending();
    });
    watcher_->setFuture(QtConcurrent::run([request, token]() -> QVariantMap {
        try {
            const auto result = echo::audio::learn_noise_profile(
                request.path.toStdString(),
                static_cast<std::uint64_t>(request.start),
                static_cast<std::uint64_t>(request.end),
                token
            );
            return token.stop_requested() ? QVariantMap{} : NoiseProfileProjection::toQml(result);
        } catch (const std::exception&) {
            return {};
        }
    }));
}

void NoiseProfileController::cancel() {
    ++generation_;
    pending_.reset();
    cancellation_.request_stop();
    running_ = false;
    error_ = 0;
    emit stateChanged();
}
