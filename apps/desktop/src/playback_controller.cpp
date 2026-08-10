//! Playback controller over Qt 6.11's callback-based QAudioSink API.
//! The audio thread fills its span from the engine ring (memcpy only); the
//! position timer (main thread) observes end-of-stream and drives UI state.

#include "playback_controller.hpp"

#include <QAudioFormat>
#include <QDebug>
#include <QMediaDevices>
#include <QSpan>
#include <algorithm>
#include <cmath>

#include "parametric_equalizer_projection.hpp"
#include "reverb_projection.hpp"

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
    int fadeInCurve,
    int fadeOutCurve,
    int gainCentibels,
    int lowCutHertz,
    const QVariantList& equalizerBands,
    bool compressorEnabled,
    int compressorThresholdCentibels,
    int compressorRatioTenths,
    int compressorAttackMillis,
    int compressorReleaseMillis,
    int compressorMakeupCentibels,
    const QVariantMap& reverbValue,
    bool limiterEnabled,
    int limiterCeilingCentibels,
    int limiterReleaseMillis
) {
    if (trimStartMillis < 0 || trimEndMillis <= trimStartMillis || fadeInMillis < 0
        || fadeOutMillis < 0 || fadeInCurve < 0 || fadeInCurve > 2 || fadeOutCurve < 0
        || fadeOutCurve > 2 || gainCentibels < -2400 || gainCentibels > 1200
        || (lowCutHertz != 0 && (lowCutHertz < 20 || lowCutHertz > 240))
        || compressorThresholdCentibels < -6000 || compressorThresholdCentibels > 0
        || compressorRatioTenths < 10 || compressorRatioTenths > 200 || compressorAttackMillis < 1
        || compressorAttackMillis > 200 || compressorReleaseMillis < 20
        || compressorReleaseMillis > 2000 || compressorMakeupCentibels < 0
        || compressorMakeupCentibels > 2400 || limiterCeilingCentibels < -600
        || limiterCeilingCentibels > 0 || limiterReleaseMillis < 20
        || limiterReleaseMillis > 1000) {
        qWarning("invalid playback adjustment");
        return;
    }
    const auto equalizer = ParametricEqualizerProjection::fromQml(equalizerBands);
    const auto reverb = ReverbProjection::fromQml(reverbValue);
    if (!equalizer.has_value() || !reverb.has_value()) {
        qWarning("invalid equalizer or reverb");
        return;
    }
    const echo::audio::PlaybackAdjustment adjustment{
        .trim_start_millis = static_cast<std::uint64_t>(trimStartMillis),
        .trim_end_millis = static_cast<std::uint64_t>(trimEndMillis),
        .fade_in_millis = static_cast<std::uint64_t>(fadeInMillis),
        .fade_out_millis = static_cast<std::uint64_t>(fadeOutMillis),
        .fade_in_curve = static_cast<echo::audio::FadeCurve>(fadeInCurve),
        .fade_out_curve = static_cast<echo::audio::FadeCurve>(fadeOutCurve),
        .gain_centibels = static_cast<std::int16_t>(gainCentibels),
        .low_cut_hertz = static_cast<std::uint16_t>(lowCutHertz),
        .equalizer = *equalizer,
        .compressor =
            {
                .enabled = compressorEnabled,
                .threshold_centibels = static_cast<std::int16_t>(compressorThresholdCentibels),
                .ratio_tenths = static_cast<std::uint16_t>(compressorRatioTenths),
                .attack_millis = static_cast<std::uint16_t>(compressorAttackMillis),
                .release_millis = static_cast<std::uint16_t>(compressorReleaseMillis),
                .makeup_centibels = static_cast<std::int16_t>(compressorMakeupCentibels),
            },
        .reverb = *reverb,
        .limiter = {
            .enabled = limiterEnabled,
            .ceiling_centibels = static_cast<std::int16_t>(limiterCeilingCentibels),
            .release_millis = static_cast<std::uint16_t>(limiterReleaseMillis),
        },
    };
    startSession(path, adjustment);
}

bool PlaybackController::updateReverb(const QVariantMap& reverbValue) {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    const auto reverb = ReverbProjection::fromQml(reverbValue);
    if (session == nullptr || !reverb.has_value()) {
        return false;
    }
    try {
        session->update_reverb(*reverb);
    } catch (const std::exception& error) {
        qWarning("cannot update playback reverb: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateEqualizer(const QVariantList& equalizerBands) {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    const auto equalizer = ParametricEqualizerProjection::fromQml(equalizerBands);
    if (session == nullptr || !equalizer.has_value()) {
        return false;
    }
    try {
        session->update_equalizer(*equalizer);
    } catch (const std::exception& error) {
        qWarning("cannot update playback equalizer: %s", error.what());
        return false;
    }
    return true;
}

QVariantList
PlaybackController::equalizerResponse(const QVariantList& equalizerBands, int pointCount) const {
    const auto equalizer = ParametricEqualizerProjection::fromQml(equalizerBands);
    return equalizer.has_value()
               ? ParametricEqualizerProjection::responseCurve(*equalizer, pointCount)
               : QVariantList{};
}

bool PlaybackController::updateCompressor(
    bool enabled,
    int thresholdCentibels,
    int ratioTenths,
    int attackMillis,
    int releaseMillis,
    int makeupCentibels
) {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    if (session == nullptr || thresholdCentibels < -6000 || thresholdCentibels > 0
        || ratioTenths < 10 || ratioTenths > 200 || attackMillis < 1 || attackMillis > 200
        || releaseMillis < 20 || releaseMillis > 2000 || makeupCentibels < 0
        || makeupCentibels > 2400) {
        return false;
    }
    try {
        session->update_compressor({
            .enabled = enabled,
            .threshold_centibels = static_cast<std::int16_t>(thresholdCentibels),
            .ratio_tenths = static_cast<std::uint16_t>(ratioTenths),
            .attack_millis = static_cast<std::uint16_t>(attackMillis),
            .release_millis = static_cast<std::uint16_t>(releaseMillis),
            .makeup_centibels = static_cast<std::int16_t>(makeupCentibels),
        });
    } catch (const std::exception& error) {
        qWarning("cannot update playback compressor: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateLimiter(bool enabled, int ceilingCentibels, int releaseMillis) {
    const std::shared_ptr<echo::audio::PlaybackSession> session = current_session_;
    if (session == nullptr || ceilingCentibels < -600 || ceilingCentibels > 0 || releaseMillis < 20
        || releaseMillis > 1000) {
        return false;
    }
    try {
        session->update_limiter({
            .enabled = enabled,
            .ceiling_centibels = static_cast<std::int16_t>(ceilingCentibels),
            .release_millis = static_cast<std::uint16_t>(releaseMillis),
        });
    } catch (const std::exception& error) {
        qWarning("cannot update playback limiter: %s", error.what());
        return false;
    }
    return true;
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
    momentary_lufs_ = -70.0;
    output_peak_db_ = -70.0;
    gain_reduction_db_ = 0.0;
    limiter_reduction_db_ = 0.0;
    position_timer_.start();
    emit stateChanged();
    emit meterChanged();
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
    momentary_lufs_ = -70.0;
    output_peak_db_ = -70.0;
    gain_reduction_db_ = 0.0;
    limiter_reduction_db_ = 0.0;
    emit stateChanged();
    emit meterChanged();
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

bool PlaybackController::isActive() const {
    return current_session_ != nullptr && !ended_;
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

qreal PlaybackController::momentaryLufs() const {
    return momentary_lufs_;
}

qreal PlaybackController::outputPeakDb() const {
    return output_peak_db_;
}

qreal PlaybackController::gainReductionDb() const {
    return gain_reduction_db_;
}

qreal PlaybackController::limiterReductionDb() const {
    return limiter_reduction_db_;
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
    if (session != nullptr) {
        const echo::audio::PlaybackMeterSnapshot snapshot = session->meter_snapshot();
        const bool changed =
            std::abs(momentary_lufs_ - snapshot.momentary_lufs) > 0.05
            || std::abs(output_peak_db_ - snapshot.output_peak_dbfs) > 0.05
            || std::abs(gain_reduction_db_ - snapshot.gain_reduction_decibels) > 0.05
            || std::abs(limiter_reduction_db_ - snapshot.limiter_reduction_decibels) > 0.05;
        if (changed) {
            momentary_lufs_ = snapshot.momentary_lufs;
            output_peak_db_ = snapshot.output_peak_dbfs;
            gain_reduction_db_ = snapshot.gain_reduction_decibels;
            limiter_reduction_db_ = snapshot.limiter_reduction_decibels;
            emit meterChanged();
        }
    }
    if (session != nullptr && session->is_ended() && session->buffered_frames() == 0 && !ended_) {
        ended_ = true;
        position_timer_.stop();
        emit stateChanged();
        return;
    }
    emit positionChanged();
}
