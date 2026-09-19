//! Latest-wins, cancellable detail projection for one spectral viewport.
#pragma once

#include "echo/audio/spectrogram_detail.hpp"
#include <QFutureWatcher>
#include <QObject>
#include <QString>
#include <optional>
#include <stop_token>

class SpectrogramPreviewController : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString scaleImageUrl READ scaleImageUrl CONSTANT)
    Q_PROPERTY(QString imageUrl READ imageUrl NOTIFY previewChanged)
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit SpectrogramPreviewController(QObject* parent = nullptr);
    ~SpectrogramPreviewController() override;
    Q_INVOKABLE void requestViewport(
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
    );
    Q_INVOKABLE void clear();
    [[nodiscard]] QString scaleImageUrl() const;
    [[nodiscard]] QString imageUrl() const;
    [[nodiscard]] bool running() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void previewChanged();
    void stateChanged();

  private:
    struct Request {
        QString path;
        QString identity;
        echo::audio::SpectrogramDetailRequest detail;
        std::uint64_t generation;
    };
    void startPending();
    QString image_url_;
    QString error_text_;
    std::uint64_t generation_ = 0;
    bool running_ = false;
    std::optional<Request> pending_;
    QFutureWatcher<QString>* watcher_ = nullptr;
    std::stop_source cancellation_;
};
