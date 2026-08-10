//! Playback controller over Qt 6.11's callback-based QAudioSink API.
//!
//! One sink is created for the controller lifetime and reused across plays:
//! disposing a CoreAudio audio unit on replay crashes Qt 6.11.1's CoreAudio
//! backend, so pause uses suspend/resume and a new recording just swaps the
//! session the callback reads from (shared_ptr-protected).

#pragma once

#include <QAudioSink>
#include <QObject>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>
#include <memory>
#include <vector>

#include "echo/audio/playback.hpp"

class PlaybackController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool playing READ isPlaying NOTIFY stateChanged)
    Q_PROPERTY(bool paused READ isPaused NOTIFY stateChanged)
    Q_PROPERTY(bool active READ isActive NOTIFY stateChanged)
    Q_PROPERTY(qint64 position READ position NOTIFY positionChanged)
    Q_PROPERTY(qint64 duration READ duration NOTIFY stateChanged)
    Q_PROPERTY(qreal volume READ volume WRITE setVolume NOTIFY volumeChanged)
    Q_PROPERTY(qreal momentaryLufs READ momentaryLufs NOTIFY meterChanged)
    Q_PROPERTY(qreal outputPeakDb READ outputPeakDb NOTIFY meterChanged)
    Q_PROPERTY(qreal gainReductionDb READ gainReductionDb NOTIFY meterChanged)
    Q_PROPERTY(qreal limiterReductionDb READ limiterReductionDb NOTIFY meterChanged)

  public:
    explicit PlaybackController(QObject* parent = nullptr);
    ~PlaybackController() override;

    Q_INVOKABLE void play(const QString& path);
    Q_INVOKABLE void playAdjusted(
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
        const QVariantList& effectChain
    );
    Q_INVOKABLE bool updateEqualizer(bool enabled, const QVariantList& equalizerBands);
    Q_INVOKABLE bool updateRestoration(const QVariantMap& restoration);
    Q_INVOKABLE bool updateDeHum(const QVariantMap& deHum);
    Q_INVOKABLE bool updateDeClick(const QVariantMap& deClick);
    Q_INVOKABLE QVariantList
    equalizerResponse(const QVariantList& equalizerBands, int pointCount) const;
    Q_INVOKABLE bool updateCompressor(
        bool enabled,
        int thresholdCentibels,
        int ratioTenths,
        int attackMillis,
        int releaseMillis,
        int makeupCentibels
    );
    Q_INVOKABLE bool updateLimiter(bool enabled, int ceilingCentibels, int releaseMillis);
    Q_INVOKABLE bool updateReverb(const QVariantMap& reverb);
    Q_INVOKABLE void togglePause();
    Q_INVOKABLE void stop();
    Q_INVOKABLE void seek(qint64 millis);

    bool isPlaying() const;
    bool isPaused() const;
    bool isActive() const;
    qint64 position() const;
    qint64 duration() const;
    qreal volume() const;
    qreal momentaryLufs() const;
    qreal outputPeakDb() const;
    qreal gainReductionDb() const;
    qreal limiterReductionDb() const;
    void setVolume(qreal volume);

  signals:
    void stateChanged();
    void positionChanged();
    void volumeChanged();
    void meterChanged();

  private:
    void startSession(const QString& path, const echo::audio::PlaybackAdjustment& adjustment);
    void pumpPosition();
    void fillBuffer(QSpan<float> buffer);

    std::shared_ptr<echo::audio::PlaybackSession> current_session_;
    // The audio callback reads this raw pointer; the session it names stays
    // alive through `current_session_` or `retired_sessions_` until the sink
    // is destroyed, so the callback can never touch a freed object.
    std::atomic<echo::audio::PlaybackSession*> callback_session_{nullptr};
    std::vector<std::shared_ptr<echo::audio::PlaybackSession>> retired_sessions_;
    std::unique_ptr<QAudioSink> sink_;
    QTimer position_timer_;
    qreal volume_ = 0.8;
    qreal momentary_lufs_ = -70.0;
    qreal output_peak_db_ = -70.0;
    qreal gain_reduction_db_ = 0.0;
    qreal limiter_reduction_db_ = 0.0;
    bool ended_ = false;
};
