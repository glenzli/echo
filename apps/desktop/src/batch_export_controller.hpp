//! Recoverable sequential delivery queue for one bounded Library projection.

#pragma once

#include <QObject>
#include <QUrl>
#include <QVariantList>

#include <atomic>
#include <cstdint>
#include <thread>

class DesktopBackend;

class BatchExportController : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap exportOptions MEMBER export_options_)
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool recoverable READ recoverable NOTIFY stateChanged)
    Q_PROPERTY(bool hasResult READ hasResult NOTIFY stateChanged)
    Q_PROPERTY(qreal progress READ progress NOTIFY progressChanged)
    Q_PROPERTY(int totalCount READ totalCount NOTIFY stateChanged)
    Q_PROPERTY(int completedCount READ completedCount NOTIFY stateChanged)
    Q_PROPERTY(int failedCount READ failedCount NOTIFY stateChanged)
    Q_PROPERTY(QString currentName READ currentName NOTIFY stateChanged)
    Q_PROPERTY(QString outputDirectory READ outputDirectory NOTIFY stateChanged)
    Q_PROPERTY(QString format READ format NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit BatchExportController(DesktopBackend& backend, QObject* parent = nullptr);
    ~BatchExportController() override;

    Q_INVOKABLE void
    start(const QVariantList& assets, const QUrl& destinationDirectory, const QString& format);
    Q_INVOKABLE void resume();
    Q_INVOKABLE void cancel();
    Q_INVOKABLE void dismiss();

    [[nodiscard]] bool running() const;
    [[nodiscard]] bool recoverable() const;
    [[nodiscard]] bool hasResult() const;
    [[nodiscard]] qreal progress() const;
    [[nodiscard]] int totalCount() const;
    [[nodiscard]] int completedCount() const;
    [[nodiscard]] int failedCount() const;
    [[nodiscard]] QString currentName() const;
    [[nodiscard]] QString outputDirectory() const;
    [[nodiscard]] QString format() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void stateChanged();
    void progressChanged();

  private:
    QVariantMap export_options_;
    void startWorker(QVariantMap manifest);
    void loadRecoveryManifest();
    void stopWorker(bool userCancelled);
    void reject(const QString& message);

    DesktopBackend& backend_;
    std::jthread worker_;
    std::atomic<std::uint64_t> generation_{0};
    std::atomic<bool> user_cancelled_{false};
    QVariantMap recovery_manifest_;
    bool running_ = false;
    bool recoverable_ = false;
    bool has_result_ = false;
    qreal progress_ = 0.0;
    int total_count_ = 0;
    int completed_count_ = 0;
    int failed_count_ = 0;
    QString current_name_;
    QString output_directory_;
    QString format_;
    QString error_text_;
};
