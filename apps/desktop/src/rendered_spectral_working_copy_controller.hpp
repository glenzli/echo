//! Asynchronous freeze lifecycle for one post-effect spectral working copy.

#pragma once

#include <QObject>
#include <QVariantMap>

#include <atomic>
#include <cstdint>
#include <thread>

class DesktopBackend;

class RenderedSpectralWorkingCopyController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool hasResult READ hasResult NOTIFY stateChanged)
    Q_PROPERTY(qreal progress READ progress NOTIFY progressChanged)
    Q_PROPERTY(qint64 workingCopyId READ workingCopyId NOTIFY stateChanged)
    Q_PROPERTY(QString cachePath READ cachePath NOTIFY stateChanged)
    Q_PROPERTY(quint32 operationCount READ operationCount NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit RenderedSpectralWorkingCopyController(
        DesktopBackend& backend,
        QObject* parent = nullptr
    );
    ~RenderedSpectralWorkingCopyController() override;

    /// Renders the currently saved complete adjustment to private cache bytes.
    Q_INVOKABLE void createFromSavedAsset(const QVariantMap& asset);
    /// Applies one full-attenuation spectral selection to the current private
    /// render and atomically publishes it into the same working layer.
    Q_INVOKABLE void eraseRegion(
        const QString& assetId,
        qint64 workingCopyId,
        const QString& cachePath,
        qint64 startMillis,
        qint64 endMillis,
        int lowHertz,
        int highHertz
    );
    Q_INVOKABLE void cancel();

    [[nodiscard]] bool running() const;
    [[nodiscard]] bool hasResult() const;
    [[nodiscard]] qreal progress() const;
    [[nodiscard]] qint64 workingCopyId() const;
    [[nodiscard]] QString cachePath() const;
    [[nodiscard]] quint32 operationCount() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void stateChanged();
    void progressChanged();

  private:
    void stopWorker();
    void reject(const QString& message);

    DesktopBackend& backend_;
    std::jthread worker_;
    std::atomic<std::uint64_t> generation_{0};
    bool running_ = false;
    bool has_result_ = false;
    qreal progress_ = 0.0;
    qint64 working_copy_id_ = 0;
    QString cache_path_;
    quint32 operation_count_ = 0;
    QString error_text_;
};
