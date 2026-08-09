//! Playback controller over Qt 6.11's callback-based QAudioSink API.
//! The audio thread fills its span from the engine ring (memcpy only); the
//! position timer (main thread) observes end-of-stream and drives UI state.

#include "playback_controller.hpp"

#include <QAudioFormat>
#include <QDebug>
#include <QMediaDevices>
#include <QSpan>
#include <algorithm>

PlaybackController::PlaybackController(QObject* parent) : QObject(parent) {
    position_timer_.setInterval(100);
    connect(&position_timer_, &QTimer::timeout, this, &PlaybackController::pumpPosition);
}

PlaybackController::~PlaybackController() {
    // Stop the sink while the session is still alive so no callback can touch
    // a destroyed session; the shared_ptr in the callback keeps the session
    // valid even if a callback is in flight.
    position_timer_.stop();
    if (sink_ != nullptr) {
        sink_->stop();
    }
    callback_session_.store(nullptr);
    current_session_.reset();
    retired_sessions_.clear();
}

void PlaybackController::play(const QString& path) {
    startSession(path, {});
}

void PlaybackController::playAdjusted(
    const QString& path,
    qint64 trimStartMillis,
    qint64 trimEndMillis,
    qint64 fadeInMillis,
    qint64 fadeOutMillis,
    int gainCentibels
) {
    if (trimStartMillis < 0 || trimEndMillis <= trimStartMillis || fadeInMillis < 0
        || fadeOutMillis < 0 || gainCentibels < -2400 || gainCentibels > 1200) {
        qWarning("invalid playback adjustment");
        return;
    }
    const echo::audio::PlaybackAdjustment adjustment{
        .trim_start_millis = static_cast<std::uint64_t>(trimStartMillis),
        .trim_end_millis = static_cast<std::uint64_t>(trimEndMillis),
        .fade_in_millis = static_cast<std::uint64_t>(fadeInMillis),
        .fade_out_millis = static_cast<std::uint64_t>(fadeOutMillis),
        .gain_centibels = static_cast<std::int16_t>(gainCentibels),
    };
    startSession(path, adjustment);
}

void PlaybackController::startSession(
    const QString& path,
    const echo::audio::PlaybackAdjustment& adjustment
) {
    std::shared_ptr<echo::audio::PlaybackSession> session;
    try {
        session = std::make_shared<echo::audio::PlaybackSession>(path.toStdString(), adjustment);
    } catch (const std::exception& error) {
        qWarning("cannot open %s: %s", qPrintable(path), error.what());
        return;
    }

    // Retire the previous session: the producer stops promptly, and the
    // object itself stays alive through the retired pool until the sink is
    // destroyed, so an in-flight audio callback can never touch freed memory.
    if (current_session_ != nullptr) {
        current_session_->stop();
        retired_sessions_.push_back(current_session_);
    }
    current_session_ = session;
    callback_session_.store(session.get());

    if (sink_ == nullptr) {
        const QAudioDevice output = QMediaDevices::defaultAudioOutput();
        QAudioFormat format;
        format.setSampleRate(static_cast<int>(session->sample_rate()));
        format.setChannelCount(static_cast<int>(session->channel_count()));
        format.setSampleFormat(QAudioFormat::Float);
        if (!output.isFormatSupported(format)) {
            qWarning(
                "audio format Float %dHz x%d unsupported",
                format.sampleRate(),
                format.channelCount()
            );
            current_session_.reset();
            callback_session_.store(nullptr);
            return;
        }
        sink_ = std::make_unique<QAudioSink>(output, format);
        sink_->setVolume(volume_);
        sink_->start([this](QSpan<float> buffer) { fillBuffer(buffer); });
    } else if (sink_->state() == QAudio::SuspendedState) {
        sink_->resume();
    }

    ended_ = false;
    position_timer_.start();
    emit stateChanged();
}

void PlaybackController::fillBuffer(QSpan<float> buffer) {
    echo::audio::PlaybackSession* const session = callback_session_.load();
    const std::size_t count = static_cast<std::size_t>(buffer.size());
    if (session == nullptr) {
        std::fill(buffer.begin(), buffer.end(), 0.0F);
        return;
    }
    // The span holds interleaved samples; the engine reads FRAMES, so the
    // frame budget is samples / channels. Passing the sample count as the
    // frame count overflows the span by `channels` (heap corruption).
    const std::size_t channels = static_cast<std::size_t>(session->channel_count());
    const std::size_t frames = session->read(buffer.data(), count / channels);
    std::fill(buffer.begin() + static_cast<qint64>(frames * channels), buffer.end(), 0.0F);
}

void PlaybackController::togglePause() {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    if (session == nullptr || sink_ == nullptr) {
        return;
    }
    if (session->is_paused()) {
        session->resume();
        sink_->resume();
    } else {
        sink_->suspend();
        session->pause();
    }
    emit stateChanged();
}

void PlaybackController::stop() {
    position_timer_.stop();
    if (sink_ != nullptr) {
        sink_->suspend();
    }
    if (current_session_ != nullptr) {
        current_session_->stop();
        retired_sessions_.push_back(current_session_);
    }
    callback_session_.store(nullptr);
    current_session_.reset();
    ended_ = false;
    emit stateChanged();
}

void PlaybackController::seek(qint64 millis) {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    if (session == nullptr || millis < 0) {
        return;
    }
    session->seek(static_cast<std::uint64_t>(millis));
    emit positionChanged();
}

bool PlaybackController::isPlaying() const {
    return current_session_ != nullptr && !current_session_->is_paused() && !ended_;
}

bool PlaybackController::isPaused() const {
    return current_session_ != nullptr && current_session_->is_paused();
}

qint64 PlaybackController::position() const {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    return session != nullptr ? static_cast<qint64>(session->position_millis()) : 0;
}

qint64 PlaybackController::duration() const {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    return session != nullptr ? static_cast<qint64>(session->duration_millis()) : 0;
}

qreal PlaybackController::volume() const {
    return volume_;
}

void PlaybackController::setVolume(qreal volume) {
    const qreal clamped = std::clamp(volume, 0.0, 1.0);
    if (qFuzzyCompare(clamped, volume_)) {
        return;
    }
    volume_ = clamped;
    if (sink_ != nullptr) {
        sink_->setVolume(volume_);
    }
    emit volumeChanged();
}

void PlaybackController::pumpPosition() {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    if (session != nullptr && session->is_ended() && session->buffered_frames() == 0 && !ended_) {
        ended_ = true;
        position_timer_.stop();
        emit stateChanged();
        return;
    }
    emit positionChanged();
}
