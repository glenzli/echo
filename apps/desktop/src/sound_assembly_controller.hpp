//! Background preparation, preview, and atomic mixdown for Sound Assembly.

#pragma once

#include <QObject>
#include <QTemporaryDir>
#include <QUrl>
#include <QVariantMap>

#include "echo/audio/assembly.hpp"
#include <atomic>
#include <cstdint>
#include <optional>
#include <thread>

class DesktopBackend;
class PlaybackController;

class SoundAssemblyController : public QObject {
    Q_OBJECT
    Q_PROPERTY(int reusedSourceCount READ reusedSourceCount NOTIFY stateChanged)
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool hasPreview READ hasPreview NOTIFY stateChanged)
    Q_PROPERTY(bool hasResult READ hasResult NOTIFY stateChanged)
    Q_PROPERTY(qreal progress READ progress NOTIFY progressChanged)
    Q_PROPERTY(QString outputPath READ outputPath NOTIFY stateChanged)
    Q_PROPERTY(qreal integratedLufs READ integratedLufs NOTIFY stateChanged)
    Q_PROPERTY(qreal truePeakDbtp READ truePeakDbtp NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit SoundAssemblyController(
        DesktopBackend& backend,
        PlaybackController& player,
        QObject* parent = nullptr
    );
    ~SoundAssemblyController() override;

    Q_INVOKABLE void preparePreview(const QVariantMap& revision);
    Q_INVOKABLE void
    prepareRangePreview(const QVariantMap& revision, qint64 startMillis, qint64 endMillis);
    Q_INVOKABLE void exportAssembly(const QVariantMap& revision, const QUrl& destination);
    Q_INVOKABLE void saveToMemory(const QVariantMap& revision);
    Q_INVOKABLE void cancel();
    Q_INVOKABLE bool playPreview(qint64 startMillis = 0);
    Q_INVOKABLE bool updatePreviewMix(const QVariantMap& revision, bool updatePlayback);

    int reusedSourceCount() const {
        return reused_source_count_;
    }
    [[nodiscard]] bool running() const;
    [[nodiscard]] bool hasPreview() const;
    [[nodiscard]] bool hasResult() const;
    [[nodiscard]] qreal progress() const;
    [[nodiscard]] QString outputPath() const;
    [[nodiscard]] qreal integratedLufs() const;
    [[nodiscard]] qreal truePeakDbtp() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void stateChanged();
    void progressChanged();
    void memorySaved(const QString& assemblyId);

  private:
    void start(
        const QVariantMap& revision,
        const QString& destination,
        bool preview,
        bool preserveMemory = false,
        qint64 startMillis = 0,
        qint64 endMillis = 0
    );
    void stopWorker();
    void reject(const QString& message);

    DesktopBackend& backend_;
    PlaybackController& player_;
    QTemporaryDir preview_directory_;
    std::jthread worker_;
    std::atomic<std::uint64_t> generation_{0};
    int reused_source_count_ = 0;
    bool running_ = false;
    bool has_preview_ = false;
    bool has_result_ = false;
    qreal progress_ = 0.0;
    std::optional<echo::audio::AssemblyMixPlan> preview_plan_;
    QVariantMap preview_revision_;
    QString output_path_;
    qreal integrated_lufs_ = -70.0;
    qreal true_peak_dbtp_ = -70.0;
    QString error_text_;
};
