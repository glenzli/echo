//! Background whole-preview loudness analysis for the adjustment workspace.

#pragma once

#include <QObject>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>
#include <cstdint>
#include <stop_token>
#include <thread>

class LoudnessAnalysisController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool hasResult READ hasResult NOTIFY stateChanged)
    Q_PROPERTY(qreal progress READ progress NOTIFY progressChanged)
    Q_PROPERTY(qreal integratedLufs READ integratedLufs NOTIFY stateChanged)
    Q_PROPERTY(qreal truePeakDbtp READ truePeakDbtp NOTIFY stateChanged)
    Q_PROPERTY(QString resultKey READ resultKey NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit LoudnessAnalysisController(QObject* parent = nullptr);
    ~LoudnessAnalysisController() override;

    Q_INVOKABLE void analyzeAdjusted(
        const QString& resultKey,
        const QString& path,
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
    Q_INVOKABLE void cancel();
    Q_INVOKABLE QVariantMap
    gainAdvice(qreal targetLufs, qreal truePeakCeilingDbtp, int currentGainCentibels) const;

    [[nodiscard]] bool running() const;
    [[nodiscard]] bool hasResult() const;
    [[nodiscard]] qreal progress() const;
    [[nodiscard]] qreal integratedLufs() const;
    [[nodiscard]] qreal truePeakDbtp() const;
    [[nodiscard]] QString resultKey() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void stateChanged();
    void progressChanged();

  private:
    void stopWorker();

    std::jthread worker_;
    std::atomic<std::uint64_t> generation_{0};
    bool running_ = false;
    bool has_result_ = false;
    qreal progress_ = 0.0;
    qreal integrated_lufs_ = -70.0;
    qreal true_peak_dbtp_ = -70.0;
    QString result_key_;
    QString error_text_;
};
