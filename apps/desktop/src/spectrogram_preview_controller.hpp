//! Request-scoped, asynchronous source spectrogram preview controller.

#pragma once

#include <QObject>
#include <QString>

#include <cstdint>

class SpectrogramPreviewController : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString assetId READ assetId NOTIFY previewChanged)
    Q_PROPERTY(QString imageUrl READ imageUrl NOTIFY previewChanged)
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit SpectrogramPreviewController(
        QString catalogPath,
        QString cacheRoot,
        QObject* parent = nullptr
    );

    Q_INVOKABLE void request(const QString& assetId);
    /// Requests an ephemeral preview for the verified active working cache.
    Q_INVOKABLE void requestPath(const QString& assetId, const QString& sourcePath);
    Q_INVOKABLE void clear();

    [[nodiscard]] QString assetId() const;
    [[nodiscard]] QString imageUrl() const;
    [[nodiscard]] bool running() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void previewChanged();
    void stateChanged();

  private:
    QString catalog_path_;
    QString cache_root_;
    QString asset_id_;
    QString image_url_;
    QString error_text_;
    std::uint64_t generation_ = 0;
    bool running_ = false;
};
