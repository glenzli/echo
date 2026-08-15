#include "spectrogram_preview_controller.hpp"

#include <QBuffer>
#include <QFutureWatcher>
#include <QImage>
#include <QVariant>
#include <QtConcurrent/QtConcurrentRun>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <utility>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

namespace {

struct PreviewOutcome {
    QString image_url;
    bool succeeded = false;
};

QRgb colorForMagnitude(std::uint8_t magnitude) {
    const double normalized = static_cast<double>(magnitude) / 255.0;
    const int red = qRound(14.0 + 241.0 * std::pow(normalized, 1.55));
    const int green = qRound(10.0 + 218.0 * std::pow(std::max(0.0, normalized - 0.30) / 0.70, 1.2));
    const int blue = qRound(30.0 + 194.0 * std::pow(std::max(0.0, normalized - 0.08) / 0.92, 0.62));
    return qRgba(red, green, blue, 255);
}

PreviewOutcome
makePreview(const QString& catalogPath, const QString& cacheRoot, const QString& assetId) {
    PreviewOutcome outcome;
    try {
        const auto artifact = echo::desktop::spectrogram_artifact_for_catalog(
            catalogPath.toStdString(),
            cacheRoot.toStdString(),
            assetId.toStdString()
        );
        const auto columns = static_cast<int>(artifact.time_columns);
        const auto bins = static_cast<int>(artifact.frequency_bins);
        if (columns <= 0 || bins <= 0
            || artifact.magnitudes.size()
                   != static_cast<std::size_t>(columns) * static_cast<std::size_t>(bins)) {
            return outcome;
        }
        QImage image(columns, bins, QImage::Format_RGBA8888);
        for (int y = 0; y < bins; ++y) {
            auto* destination = image.scanLine(y);
            const int source_bin = bins - y - 1;
            for (int x = 0; x < columns; ++x) {
                const std::size_t offset =
                    static_cast<std::size_t>(x) * static_cast<std::size_t>(bins)
                    + static_cast<std::size_t>(source_bin);
                const QRgb color = colorForMagnitude(artifact.magnitudes[offset]);
                const int destination_offset = x * 4;
                destination[destination_offset] = static_cast<uchar>(qRed(color));
                destination[destination_offset + 1] = static_cast<uchar>(qGreen(color));
                destination[destination_offset + 2] = static_cast<uchar>(qBlue(color));
                destination[destination_offset + 3] = static_cast<uchar>(qAlpha(color));
            }
        }
        QByteArray encoded;
        QBuffer buffer(&encoded);
        if (!buffer.open(QIODevice::WriteOnly) || !image.save(&buffer, "PNG")) {
            return outcome;
        }
        outcome.image_url =
            QStringLiteral("data:image/png;base64,") + QString::fromLatin1(encoded.toBase64());
        outcome.succeeded = true;
    } catch (const rust::Error&) {
        // The QML projection has an explicit unavailable state; keep source
        // and catalog paths out of desktop diagnostics.
    }
    return outcome;
}

} // namespace

SpectrogramPreviewController::SpectrogramPreviewController(
    QString catalogPath,
    QString cacheRoot,
    QObject* parent
) : QObject(parent), catalog_path_(std::move(catalogPath)), cache_root_(std::move(cacheRoot)) {}

void SpectrogramPreviewController::request(const QString& assetId) {
    if (assetId.isEmpty()) {
        clear();
        return;
    }
    const std::uint64_t generation = ++generation_;
    asset_id_ = assetId;
    image_url_.clear();
    error_text_.clear();
    running_ = true;
    emit previewChanged();
    emit stateChanged();

    const QString catalog_path = catalog_path_;
    const QString cache_root = cache_root_;
    auto* watcher = new QFutureWatcher<PreviewOutcome>(this);
    connect(watcher, &QFutureWatcher<PreviewOutcome>::finished, this, [this, watcher, generation] {
        const PreviewOutcome outcome = watcher->result();
        watcher->deleteLater();
        if (generation != generation_) {
            return;
        }
        running_ = false;
        if (outcome.succeeded) {
            image_url_ = outcome.image_url;
            error_text_.clear();
        } else {
            image_url_.clear();
            error_text_ = QStringLiteral("spectrogram overview unavailable");
        }
        emit previewChanged();
        emit stateChanged();
    });
    watcher->setFuture(QtConcurrent::run([catalog_path, cache_root, assetId] {
        return makePreview(catalog_path, cache_root, assetId);
    }));
}

void SpectrogramPreviewController::clear() {
    ++generation_;
    const bool preview_changed = !asset_id_.isEmpty() || !image_url_.isEmpty();
    const bool state_changed = running_ || !error_text_.isEmpty();
    asset_id_.clear();
    image_url_.clear();
    error_text_.clear();
    running_ = false;
    if (preview_changed) {
        emit previewChanged();
    }
    if (state_changed) {
        emit stateChanged();
    }
}

QString SpectrogramPreviewController::assetId() const {
    return asset_id_;
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
