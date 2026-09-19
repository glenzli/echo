#include "spectrogram_preview_controller.hpp"
#include "spectrogram_palette.hpp"

#include <QBuffer>
#include <QImage>
#include <QtConcurrent/QtConcurrentRun>
#include <algorithm>
#include <cmath>

namespace {
QString makePreview(const echo::audio::SpectrogramDetail& detail) {
    QImage image(
        static_cast<int>(detail.columns),
        static_cast<int>(detail.rows),
        QImage::Format_RGBA8888
    );
    for (std::uint32_t y = 0; y < detail.rows; ++y) {
        auto* destination = image.scanLine(static_cast<int>(y));
        for (std::uint32_t x = 0; x < detail.columns; ++x) {
            const auto value =
                detail.magnitudes[static_cast<std::size_t>(x) * detail.rows + detail.rows - y - 1];
            const auto color = SpectrogramPalette::color(value);
            std::copy(color.begin(), color.end(), destination + x * 4);
        }
    }
    QByteArray encoded;
    QBuffer buffer(&encoded);
    if (!buffer.open(QIODevice::WriteOnly) || !image.save(&buffer, "PNG"))
        return {};
    return QStringLiteral("data:image/png;base64,") + QString::fromLatin1(encoded.toBase64());
}
} // namespace

SpectrogramPreviewController::SpectrogramPreviewController(QObject* parent) : QObject(parent) {}
SpectrogramPreviewController::~SpectrogramPreviewController() {
    cancellation_.request_stop();
    if (watcher_)
        watcher_->waitForFinished();
}

void SpectrogramPreviewController::requestViewport(
    const QString& sourcePath,
    const QString& sourceIdentity,
    qint64 startMillis,
    qint64 endMillis,
    double lowHertz,
    double highHertz,
    bool logarithmic,
    int windowFrames,
    int floorDecibels,
    int ceilingDecibels
) {
    if (sourcePath.isEmpty() || sourceIdentity.isEmpty() || startMillis < 0
        || endMillis <= startMillis) {
        clear();
        return;
    }
    pending_ = Request{
        sourcePath,
        sourceIdentity,
        {static_cast<std::uint64_t>(startMillis),
         static_cast<std::uint64_t>(endMillis),
         lowHertz,
         highHertz,
         logarithmic,
         static_cast<std::uint32_t>(windowFrames),
         1'024,
         384,
         static_cast<float>(floorDecibels),
         static_cast<float>(ceilingDecibels)},
        ++generation_
    };
    cancellation_.request_stop();
    image_url_.clear();
    error_text_.clear();
    running_ = true;
    emit previewChanged();
    emit stateChanged();
    startPending();
}

void SpectrogramPreviewController::startPending() {
    if (watcher_ || !pending_)
        return;
    const auto request = *pending_;
    pending_.reset();
    cancellation_ = std::stop_source{};
    const auto token = cancellation_.get_token();
    watcher_ = new QFutureWatcher<QString>(this);
    connect(watcher_, &QFutureWatcher<QString>::finished, this, [this, request] {
        const QString result = watcher_->result();
        watcher_->deleteLater();
        watcher_ = nullptr;
        if (request.generation == generation_) {
            image_url_ = result;
            error_text_ = result.isEmpty() ? QStringLiteral("spectrogram unavailable") : QString{};
            running_ = false;
            emit previewChanged();
            emit stateChanged();
        }
        startPending();
    });
    watcher_->setFuture(QtConcurrent::run([request, token]() -> QString {
        try {
            const auto detail = echo::audio::build_spectrogram_detail(
                request.path.toStdString(),
                request.detail,
                token
            );
            return token.stop_requested() ? QString{} : makePreview(detail);
        } catch (const std::exception&) {
            return {};
        }
    }));
}

void SpectrogramPreviewController::clear() {
    ++generation_;
    pending_.reset();
    cancellation_.request_stop();
    image_url_.clear();
    error_text_.clear();
    running_ = false;
    emit previewChanged();
    emit stateChanged();
}
QString SpectrogramPreviewController::imageUrl() const {
    return image_url_;
}
bool SpectrogramPreviewController::running() const {
    return running_;
}
QString SpectrogramPreviewController::errorText() const {
    return error_text_;
}

QString SpectrogramPreviewController::scaleImageUrl() const {
    static const QString scale = [] {
        echo::audio::SpectrogramDetail detail{256, 1, std::vector<std::uint8_t>(256)};
        for (std::size_t index = 0; index < 256; ++index)
            detail.magnitudes[index] = static_cast<std::uint8_t>(index);
        return makePreview(detail);
    }();
    return scale;
}
