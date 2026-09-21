//! Asynchronous, atomic publication controller for one offline WAV render.

#pragma once

#include <QObject>
#include <QUrl>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>
#include <cstdint>
#include <thread>

class DesktopBackend;

class RenderExportController : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap exportOptions MEMBER export_options_)
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool hasResult READ hasResult NOTIFY stateChanged)
    Q_PROPERTY(qreal progress READ progress NOTIFY progressChanged)
    Q_PROPERTY(QString outputPath READ outputPath NOTIFY stateChanged)
    Q_PROPERTY(qreal integratedLufs READ integratedLufs NOTIFY stateChanged)
    Q_PROPERTY(qreal truePeakDbtp READ truePeakDbtp NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit RenderExportController(DesktopBackend& backend, QObject* parent = nullptr);
    ~RenderExportController() override;

    // Preserves the existing QML call until the Creative draft is connected.
    Q_INVOKABLE void exportAdjusted(
        const QString& assetId,
        qint64 adjustmentRevisionId,
        const QString& sourcePath,
        const QUrl& destination,
        qint64 trimStartMillis,
        qint64 trimEndMillis,
        qint64 fadeInMillis,
        qint64 fadeOutMillis,
        int fadeInCurve,
        int fadeOutCurve,
        int gainCentibels,
        int lowCutHertz,
        const QVariantMap& restoration,
        const QVariantMap& deHum,
        const QVariantMap& deClick,
        const QVariantMap& channelRepair,
        bool equalizerEnabled,
        const QVariantList& equalizerBands,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels,
        const QVariantMap& reverb,
        bool limiterEnabled,
        int limiterCeilingCentibels,
        int limiterReleaseMillis,
        const QVariantList& effectChain,
        const QVariantList& editSegments,
        const QVariantList& effectMasks
    );
    Q_INVOKABLE void exportAdjusted(
        const QString& assetId,
        qint64 adjustmentRevisionId,
        const QString& sourcePath,
        const QUrl& destination,
        qint64 trimStartMillis,
        qint64 trimEndMillis,
        qint64 fadeInMillis,
        qint64 fadeOutMillis,
        int fadeInCurve,
        int fadeOutCurve,
        int gainCentibels,
        int lowCutHertz,
        const QVariantMap& restoration,
        const QVariantMap& deHum,
        const QVariantMap& deClick,
        const QVariantMap& channelRepair,
        bool equalizerEnabled,
        const QVariantList& equalizerBands,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels,
        const QVariantMap& reverb,
        bool limiterEnabled,
        int limiterCeilingCentibels,
        int limiterReleaseMillis,
        const QVariantList& effectChain,
        const QVariantList& editSegments,
        const QVariantList& effectMasks,
        const QVariantMap& creativeVfx,
        const QVariantMap& spectralRepair = QVariantMap{}
    );
    /// Publishes the active rendered repair copy itself. It deliberately uses
    /// an empty downstream adjustment so post-freeze effects are never applied
    /// twice.
    Q_INVOKABLE void exportRenderedSpectralWorkingCopy(
        const QString& assetId,
        qint64 adjustmentRevisionId,
        qint64 workingCopyId,
        const QString& renderedSourcePath,
        const QUrl& destination
    );
    Q_INVOKABLE void cancel();

    [[nodiscard]] bool running() const;
    [[nodiscard]] bool hasResult() const;
    [[nodiscard]] qreal progress() const;
    [[nodiscard]] QString outputPath() const;
    [[nodiscard]] qreal integratedLufs() const;
    [[nodiscard]] qreal truePeakDbtp() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void stateChanged();
    void progressChanged();

  private:
    QVariantMap export_options_;
    void stopWorker();
    void reject(const QString& message);

    DesktopBackend& backend_;
    std::jthread worker_;
    std::atomic<std::uint64_t> generation_{0};
    bool running_ = false;
    bool has_result_ = false;
    qreal progress_ = 0.0;
    QString output_path_;
    qreal integrated_lufs_ = -70.0;
    qreal true_peak_dbtp_ = -70.0;
    QString error_text_;
};
