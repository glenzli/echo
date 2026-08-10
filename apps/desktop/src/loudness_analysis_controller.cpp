#include "loudness_analysis_controller.hpp"

#include <QMetaObject>

#include <algorithm>
#include <chrono>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

#include "echo/audio/loudness_gain_advisor.hpp"
#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/playback.hpp"
#include "playback_adjustment_projection.hpp"

LoudnessAnalysisController::LoudnessAnalysisController(QObject* parent) : QObject(parent) {}

LoudnessAnalysisController::~LoudnessAnalysisController() {
    stopWorker();
}

void LoudnessAnalysisController::analyzeAdjusted(
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
    int limiterReleaseMillis
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
        equalizerBands,
        compressorEnabled,
        compressorThresholdCentibels,
        compressorRatioTenths,
        compressorAttackMillis,
        compressorReleaseMillis,
        compressorMakeupCentibels,
        reverb,
        limiterEnabled,
        limiterCeilingCentibels,
        limiterReleaseMillis
    );
    if (!adjustment.has_value()) {
        error_text_ = QStringLiteral("adjustment is outside the supported range");
        has_result_ = false;
        emit stateChanged();
        return;
    }

    stopWorker();
    const std::uint64_t generation = generation_.fetch_add(1) + 1;
    running_ = true;
    has_result_ = false;
    progress_ = 0.0;
    result_key_.clear();
    error_text_.clear();
    emit stateChanged();
    emit progressChanged();

    const std::string source_path = path.toStdString();
    const std::uint64_t analysis_start = adjustment->trim_start_millis;
    const std::uint64_t analysis_duration =
        adjustment->trim_end_millis - adjustment->trim_start_millis;
    worker_ = std::jthread([this,
                            generation,
                            resultKey,
                            source_path,
                            adjustment = *adjustment,
                            analysis_start,
                            analysis_duration](std::stop_token stop_token) {
        try {
            echo::audio::PlaybackSession session(source_path, adjustment);
            echo::audio::OfflineLoudnessAnalyzer analyzer(
                session.sample_rate(),
                session.channel_count()
            );
            std::vector<float> buffer(4096 * session.channel_count());
            auto last_progress = std::chrono::steady_clock::now();
            while (!stop_token.stop_requested()) {
                const std::size_t frames = session.read(buffer.data(), 4096);
                if (frames > 0) {
                    analyzer.process_interleaved(buffer.data(), frames, session.channel_count());
                } else if (session.is_ended() && session.buffered_frames() == 0) {
                    break;
                } else {
                    std::this_thread::sleep_for(std::chrono::milliseconds(1));
                }
                const auto now = std::chrono::steady_clock::now();
                if (now - last_progress >= std::chrono::milliseconds(100)) {
                    last_progress = now;
                    const std::uint64_t position = session.position_millis();
                    const std::uint64_t completed =
                        position > analysis_start ? position - analysis_start : 0;
                    const qreal value = analysis_duration > 0
                                            ? std::clamp(
                                                  static_cast<qreal>(completed)
                                                      / static_cast<qreal>(analysis_duration),
                                                  0.0,
                                                  1.0
                                              )
                                            : 0.0;
                    QMetaObject::invokeMethod(
                        this,
                        [this, generation, value] {
                            if (generation_.load() == generation) {
                                progress_ = value;
                                emit progressChanged();
                            }
                        },
                        Qt::QueuedConnection
                    );
                }
            }
            session.stop();
            if (stop_token.stop_requested()) {
                return;
            }
            const echo::audio::OfflineLoudnessResult result = analyzer.result();
            QMetaObject::invokeMethod(
                this,
                [this, generation, resultKey, result] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = true;
                    progress_ = 1.0;
                    integrated_lufs_ = result.integrated_lufs;
                    true_peak_dbtp_ = result.true_peak_dbtp;
                    result_key_ = resultKey;
                    emit progressChanged();
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        } catch (const std::exception& error) {
            const QString message = QString::fromUtf8(error.what());
            QMetaObject::invokeMethod(
                this,
                [this, generation, message] {
                    if (generation_.load() != generation) {
                        return;
                    }
                    running_ = false;
                    has_result_ = false;
                    error_text_ = message;
                    emit stateChanged();
                },
                Qt::QueuedConnection
            );
        }
    });
}

void LoudnessAnalysisController::cancel() {
    stopWorker();
    generation_.fetch_add(1);
    if (running_) {
        running_ = false;
        emit stateChanged();
    }
}

QVariantMap LoudnessAnalysisController::gainAdvice(
    qreal targetLufs,
    qreal truePeakCeilingDbtp,
    int currentGainCentibels
) const {
    QVariantMap projection{
        {QStringLiteral("available"), false},
        {QStringLiteral("peakConstrained"), false},
        {QStringLiteral("gainRangeConstrained"), false},
        {QStringLiteral("targetReached"), false},
        {QStringLiteral("gainDeltaCentibels"), 0},
        {QStringLiteral("resultingGainCentibels"), currentGainCentibels},
        {QStringLiteral("estimatedIntegratedLufs"), integrated_lufs_},
        {QStringLiteral("estimatedTruePeakDbtp"), true_peak_dbtp_},
    };
    if (!has_result_ || currentGainCentibels < -2400 || currentGainCentibels > 1200) {
        return projection;
    }
    try {
        const echo::audio::LoudnessGainAdvice advice = echo::audio::LoudnessGainAdvisor::advise({
            .integrated_lufs = static_cast<float>(integrated_lufs_),
            .true_peak_dbtp = static_cast<float>(true_peak_dbtp_),
            .target_lufs = static_cast<float>(targetLufs),
            .true_peak_ceiling_dbtp = static_cast<float>(truePeakCeilingDbtp),
            .current_gain_centibels = static_cast<std::int16_t>(currentGainCentibels),
        });
        projection[QStringLiteral("available")] = advice.available;
        projection[QStringLiteral("peakConstrained")] = advice.peak_constrained;
        projection[QStringLiteral("gainRangeConstrained")] = advice.gain_range_constrained;
        projection[QStringLiteral("targetReached")] = advice.target_reached;
        projection[QStringLiteral("gainDeltaCentibels")] = advice.gain_delta_centibels;
        projection[QStringLiteral("resultingGainCentibels")] = advice.resulting_gain_centibels;
        projection[QStringLiteral("estimatedIntegratedLufs")] = advice.estimated_integrated_lufs;
        projection[QStringLiteral("estimatedTruePeakDbtp")] = advice.estimated_true_peak_dbtp;
    } catch (const std::exception&) {
        // QML callers receive an unavailable recommendation rather than a
        // cross-language exception for a transient or stale parameter set.
    }
    return projection;
}

bool LoudnessAnalysisController::running() const {
    return running_;
}
bool LoudnessAnalysisController::hasResult() const {
    return has_result_;
}
qreal LoudnessAnalysisController::progress() const {
    return progress_;
}
qreal LoudnessAnalysisController::integratedLufs() const {
    return integrated_lufs_;
}
qreal LoudnessAnalysisController::truePeakDbtp() const {
    return true_peak_dbtp_;
}
QString LoudnessAnalysisController::resultKey() const {
    return result_key_;
}
QString LoudnessAnalysisController::errorText() const {
    return error_text_;
}

void LoudnessAnalysisController::stopWorker() {
    if (worker_.joinable()) {
        worker_.request_stop();
        worker_.join();
    }
}
