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
        int eqLowGainCentibels,
        int eqMidGainCentibels,
        int eqHighGainCentibels,
        bool compressorEnabled,
        int compressorThresholdCentibels,
        int compressorRatioTenths,
        int compressorAttackMillis,
        int compressorReleaseMillis,
        int compressorMakeupCentibels
    );
    Q_INVOKABLE bool
    updateEqualizer(int eqLowGainCentibels, int eqMidGainCentibels, int eqHighGainCentibels);
    Q_INVOKABLE bool updateCompressor(
        bool enabled,
        int thresholdCentibels,
        int ratioTenths,
        int attackMillis,
        int releaseMillis,
        int makeupCentibels
    );
    Q_INVOKABLE void togglePause();
    Q_INVOKABLE void stop();
    Q_INVOKABLE void seek(qint64 millis);

    bool isPlaying() const;
    bool isPaused() const;
    bool isActive() const;
    qint64 position() const;
    qint64 duration() const;
    qreal volume() const;
    void setVolume(qreal volume);

  signals:
    void stateChanged();
    void positionChanged();
    void volumeChanged();

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
    bool ended_ = false;
};
