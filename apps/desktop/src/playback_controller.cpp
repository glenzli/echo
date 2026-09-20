//! Playback controller over Qt 6.11's callback-based QAudioSink API.
//! The audio thread fills its span from the engine ring (memcpy only); the
//! position timer (main thread) observes end-of-stream and drives UI state.

#include "playback_controller.hpp"
#include "echo/audio/assembly_playback.hpp"
#include "noise_profile_projection.hpp"

#include <QAudioFormat>
#include <QDebug>
#include <QMediaDevices>
#include <QSpan>
#include <algorithm>
#include <cmath>

#include "creative_vfx_projection.hpp"
#include "parametric_equalizer_projection.hpp"
#include "playback_adjustment_projection.hpp"
#include "restoration_projection.hpp"
#include "reverb_projection.hpp"
#include "space_projection.hpp"

PlaybackController::PlaybackController(QObject* parent) : QObject(parent) {
    position_timer_.setInterval(100);
    connect(&position_timer_, &QTimer::timeout, this, &PlaybackController::pumpPosition);
    session_cleanup_timer_.setInterval(100);
    connect(&session_cleanup_timer_, &QTimer::timeout, this, [this] {
        if (callback_sessions_.collectRetired())
            session_cleanup_timer_.stop();
    });
}

PlaybackController::~PlaybackController() {
    // Quiesce the device before releasing the callback's borrowing owner.
    position_timer_.stop();
    session_cleanup_timer_.stop();
    if (sink_ != nullptr) {
        sink_->stop();
        sink_.reset();
    }
    current_session_.reset();
    (void)callback_sessions_.publish(nullptr);
}

void PlaybackController::play(const QString& path) {
    startSession(path, {});
}

void PlaybackController::playNoiseResidue(
    const QString& path,
    qint64 startMillis,
    qint64 endMillis,
    const QVariantMap& profile
) {
    auto noise = NoiseProfileProjection::fromQml(profile);
    if (!noise || startMillis < 0 || endMillis <= startMillis)
        return;
    noise->enabled = true;
    noise->residue = true;
    echo::audio::PlaybackAdjustment adjustment;
    adjustment.trim_start_millis = static_cast<std::uint64_t>(startMillis);
    adjustment.trim_end_millis = static_cast<std::uint64_t>(endMillis);
    adjustment.profiled_noise_reduction = std::move(noise);
    startSession(path, adjustment);
}

void PlaybackController::playSpectralBand(
    const QString& path,
    qint64 startMillis,
    qint64 endMillis,
    int lowHertz,
    int highHertz
) {
    if (startMillis < 0 || endMillis <= startMillis || lowHertz < 20 || highHertz > 24'000
        || lowHertz >= highHertz)
        return;
    echo::audio::PlaybackAdjustment adjustment;
    adjustment.trim_start_millis = static_cast<std::uint64_t>(startMillis);
    adjustment.trim_end_millis = static_cast<std::uint64_t>(endMillis);
    adjustment.fade_in_millis = std::min<qint64>(5, (endMillis - startMillis) / 2);
    adjustment.fade_out_millis = adjustment.fade_in_millis;
    if (lowHertz > 20)
        adjustment.spectral_repair.push_back(
            {0,
             static_cast<std::uint64_t>(endMillis),
             20,
             static_cast<std::uint16_t>(lowHertz),
             9600,
             0,
             0}
        );
    if (highHertz < 24'000)
        adjustment.spectral_repair.push_back(
            {0,
             static_cast<std::uint64_t>(endMillis),
             static_cast<std::uint16_t>(highHertz),
             24'000,
             9600,
             0,
             0}
        );
    startSession(path, adjustment);
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
    const QVariantMap& restorationValue,
    const QVariantMap& deHumValue,
    const QVariantMap& deClickValue,
    const QVariantMap& channelRepairValue,
    bool equalizerEnabled,
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
    int limiterReleaseMillis,
    const QVariantList& effectChain,
    const QVariantList& editSegments,
    const QVariantList& effectMasks
) {
    playAdjusted(
        path,
        trimStartMillis,
        trimEndMillis,
        fadeInMillis,
        fadeOutMillis,
        fadeInCurve,
        fadeOutCurve,
        gainCentibels,
        lowCutHertz,
        restorationValue,
        deHumValue,
        deClickValue,
        channelRepairValue,
        equalizerEnabled,
        equalizerBands,
        compressorEnabled,
        compressorThresholdCentibels,
        compressorRatioTenths,
        compressorAttackMillis,
        compressorReleaseMillis,
        compressorMakeupCentibels,
        reverbValue,
        limiterEnabled,
        limiterCeilingCentibels,
        limiterReleaseMillis,
        effectChain,
        editSegments,
        effectMasks,
        {}
    );
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
    const QVariantMap& restorationValue,
    const QVariantMap& deHumValue,
    const QVariantMap& deClickValue,
    const QVariantMap& channelRepairValue,
    bool equalizerEnabled,
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
    int limiterReleaseMillis,
    const QVariantList& effectChain,
    const QVariantList& editSegments,
    const QVariantList& effectMasks,
    const QVariantMap& creativeVfxValue,
    const QVariantMap& spectralRepair
) {
    const auto adjustment = PlaybackAdjustmentProjection::fromQml(
        trimStartMillis,
        trimEndMillis,
        fadeInMillis,
        fadeOutMillis,
        fadeInCurve,
        fadeOutCurve,
        gainCentibels,
        lowCutHertz,
        restorationValue,
        deHumValue,
        deClickValue,
        channelRepairValue,
        equalizerEnabled,
        equalizerBands,
        compressorEnabled,
        compressorThresholdCentibels,
        compressorRatioTenths,
        compressorAttackMillis,
        compressorReleaseMillis,
        compressorMakeupCentibels,
        reverbValue,
        limiterEnabled,
        limiterCeilingCentibels,
        limiterReleaseMillis,
        effectChain,
        editSegments,
        effectMasks,
        creativeVfxValue,
        spectralRepair
    );
    if (!adjustment.has_value()) {
        qWarning("invalid playback adjustment");
        return;
    }
    startSession(path, *adjustment);
}

bool PlaybackController::updateRestoration(const QVariantMap& restorationValue) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    const auto restoration = RestorationProjection::fromQml(restorationValue);
    if (session == nullptr || !restoration.has_value()) {
        return false;
    }
    try {
        session->update_restoration(*restoration);
    } catch (const std::exception& error) {
        qWarning("cannot update playback restoration: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateDeHum(const QVariantMap& deHumValue) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    const auto adjustment = PlaybackAdjustmentProjection::deHumFromQml(deHumValue);
    if (session == nullptr || !adjustment.has_value()) {
        return false;
    }
    try {
        session->update_de_hum(*adjustment);
    } catch (const std::exception& error) {
        qWarning("cannot update playback de-hum: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateDeClick(const QVariantMap& deClickValue) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    const auto adjustment = PlaybackAdjustmentProjection::deClickFromQml(deClickValue);
    if (session == nullptr || !adjustment.has_value()) {
        return false;
    }
    try {
        session->update_de_click(*adjustment);
    } catch (const std::exception& error) {
        qWarning("cannot update playback de-click: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateChannelRepair(const QVariantMap& channelRepairValue) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    const auto adjustment = PlaybackAdjustmentProjection::channelRepairFromQml(channelRepairValue);
    if (session == nullptr || !adjustment.has_value()) {
        return false;
    }
    try {
        session->update_channel_repair(*adjustment);
    } catch (const std::exception& error) {
        qWarning("cannot update playback channel repair: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateReverb(const QVariantMap& reverbValue) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    const auto reverb = ReverbProjection::fromQml(reverbValue);
    const auto space =
        reverb.has_value() ? SpaceProjection::fromQml(reverbValue, *reverb) : std::nullopt;
    if (session == nullptr || !space.has_value()) {
        return false;
    }
    try {
        session->update_space(*space);
    } catch (const std::exception& error) {
        qWarning("cannot update playback reverb: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateCreativeVfx(const QVariantMap& creativeVfxValue) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    const auto creative_vfx = CreativeVfxProjection::fromQml(creativeVfxValue);
    if (session == nullptr || !creative_vfx.has_value()) {
        return false;
    }
    try {
        session->update_creative_vfx(*creative_vfx);
    } catch (const std::exception& error) {
        qWarning("cannot update playback creative VFX: %s", error.what());
        return false;
    }
    return true;
}

bool PlaybackController::updateEqualizer(bool enabled, const QVariantList& equalizerBands) {
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
    auto equalizer = ParametricEqualizerProjection::fromQml(equalizerBands);
    if (session == nullptr || !equalizer.has_value()) {
        return false;
    }
    equalizer->enabled = enabled;
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
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
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
    const auto session = std::dynamic_pointer_cast<echo::audio::PlaybackSession>(current_session_);
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

    startStream(std::move(session));
}

bool PlaybackController::playAssembly(const echo::audio::AssemblyMixPlan& plan) {
    try {
        return startStream(std::make_shared<echo::audio::AssemblyPlaybackSession>(plan));
    } catch (const std::exception& error) {
        qWarning("cannot start assembly playback: %s", error.what());
        error_text_ = tr("Could not play the mix. Check that its sources are available.");
        emit stateChanged();
        return false;
    }
}

bool PlaybackController::startStream(std::shared_ptr<echo::audio::PlaybackStream> session) {
    error_text_.clear();
    // Stop its producer before publication. The handoff preserves an old
    // session only while an in-flight callback might still read from it.
    if (current_session_ != nullptr) {
        current_session_->stop();
    }
    publishSession(session);

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
            publishSession(nullptr);
            error_text_ = tr("The audio output device does not support this format.");
            emit stateChanged();
            return false;
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
    return true;
}

void PlaybackController::publishSession(std::shared_ptr<echo::audio::PlaybackStream> session) {
    current_session_ = session;
    if (callback_sessions_.publish(std::move(session)))
        session_cleanup_timer_.stop();
    else
        session_cleanup_timer_.start();
}

void PlaybackController::fillBuffer(QSpan<float> buffer) {
    const auto read = callback_sessions_.read();
    echo::audio::PlaybackStream* const session = read.session();
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
    const auto session = current_session_;
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
    }
    publishSession(nullptr);
    ended_ = false;
    momentary_lufs_ = -70.0;
    output_peak_db_ = -70.0;
    gain_reduction_db_ = 0.0;
    limiter_reduction_db_ = 0.0;
    emit stateChanged();
    emit meterChanged();
}

void PlaybackController::seek(qint64 millis) {
    const auto session = current_session_;
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
    const auto session = current_session_;
    return session != nullptr ? static_cast<qint64>(session->position_millis()) : 0;
}

qint64 PlaybackController::duration() const {
    const auto session = current_session_;
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
    const auto session = current_session_;
    if (session && !session->error().empty()) {
        qWarning("assembly playback failed: %s", session->error().c_str());
        error_text_ = tr("Could not play the mix. Check that its sources are available.");
        stop();
        return;
    }
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
