#include "impulse_response_controller.hpp"

#include "desktop_backend.hpp"

#include <QMetaObject>

ImpulseResponseController::ImpulseResponseController(DesktopBackend& backend, QObject* parent) :
    QObject(parent), backend_(backend) {}

ImpulseResponseController::~ImpulseResponseController() {
    generation_.fetch_add(1, std::memory_order_relaxed);
    if (worker_.joinable()) {
        worker_.request_stop();
        worker_.join();
    }
}

void ImpulseResponseController::importLocalWav(
    const QString& sourcePath,
    const QString& displayName,
    const QString& creator,
    const QString& sourceUrl,
    const QString& attribution,
    const QString& rightsKind,
    const QString& spdxExpression,
    const QString& licenseUrl
) {
    importLocalWavWithLayout(
        sourcePath,
        QStringLiteral("mono_or_stereo"),
        displayName,
        creator,
        sourceUrl,
        attribution,
        rightsKind,
        spdxExpression,
        licenseUrl
    );
}

void ImpulseResponseController::importLocalWavWithLayout(
    const QString& sourcePath,
    const QString& preparationLayout,
    const QString& displayName,
    const QString& creator,
    const QString& sourceUrl,
    const QString& attribution,
    const QString& rightsKind,
    const QString& spdxExpression,
    const QString& licenseUrl
) {
    if (busy_) {
        return;
    }
    busy_ = true;
    error_text_.clear();
    emit stateChanged();
    const std::uint64_t generation = generation_.fetch_add(1, std::memory_order_relaxed) + 1;
    worker_ = std::jthread([this,
                            generation,
                            sourcePath,
                            preparationLayout,
                            displayName,
                            creator,
                            sourceUrl,
                            attribution,
                            rightsKind,
                            spdxExpression,
                            licenseUrl](std::stop_token stop) {
        const QVariantMap result = backend_.importImpulseResponseWithLayout(
            sourcePath,
            preparationLayout,
            displayName,
            creator,
            sourceUrl,
            attribution,
            rightsKind,
            spdxExpression,
            licenseUrl
        );
        if (stop.stop_requested()) {
            return;
        }
        QMetaObject::invokeMethod(
            this,
            [this, generation, result] {
                if (generation != generation_.load(std::memory_order_relaxed)) {
                    return;
                }
                busy_ = false;
                error_text_ = result.value(QStringLiteral("error")).toString();
                if (error_text_.isEmpty()) {
                    impulse_responses_.prepend(result);
                }
                emit stateChanged();
                if (error_text_.isEmpty()) {
                    emit imported(result);
                }
            },
            Qt::QueuedConnection
        );
    });
}

void ImpulseResponseController::refresh() {
    if (busy_) {
        return;
    }
    busy_ = true;
    error_text_.clear();
    emit stateChanged();
    const std::uint64_t generation = generation_.fetch_add(1, std::memory_order_relaxed) + 1;
    worker_ = std::jthread([this, generation](std::stop_token stop) {
        const QVariantList result = backend_.listImpulseResponses();
        if (stop.stop_requested()) {
            return;
        }
        QMetaObject::invokeMethod(
            this,
            [this, generation, result] {
                if (generation != generation_.load(std::memory_order_relaxed)) {
                    return;
                }
                busy_ = false;
                impulse_responses_ = result;
                emit stateChanged();
            },
            Qt::QueuedConnection
        );
    });
}

bool ImpulseResponseController::busy() const {
    return busy_;
}

QString ImpulseResponseController::errorText() const {
    return error_text_;
}

QVariantList ImpulseResponseController::impulseResponses() const {
    return impulse_responses_;
}
