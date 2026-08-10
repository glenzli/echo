#include "echo/audio/loudness_gain_advisor.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr float kSilenceFloorLufs = -69.9F;
constexpr float kMinimumTargetLufs = -36.0F;
constexpr float kMaximumTargetLufs = -5.0F;
constexpr float kMinimumTruePeakCeilingDbtp = -12.0F;
constexpr float kMaximumTruePeakCeilingDbtp = 0.0F;
constexpr std::int16_t kMinimumGainCentibels = -2400;
constexpr std::int16_t kMaximumGainCentibels = 1200;
constexpr int kGainStepCentibels = 10;
constexpr float kReachedToleranceLufs = 0.15F;

bool finite(float value) {
    return std::isfinite(value);
}

} // namespace

LoudnessGainAdvice LoudnessGainAdvisor::advise(LoudnessGainRequest request) {
    if (!finite(request.integrated_lufs) || !finite(request.true_peak_dbtp)
        || !finite(request.target_lufs) || !finite(request.true_peak_ceiling_dbtp)
        || request.target_lufs < kMinimumTargetLufs || request.target_lufs > kMaximumTargetLufs
        || request.true_peak_ceiling_dbtp < kMinimumTruePeakCeilingDbtp
        || request.true_peak_ceiling_dbtp > kMaximumTruePeakCeilingDbtp
        || request.current_gain_centibels < kMinimumGainCentibels
        || request.current_gain_centibels > kMaximumGainCentibels) {
        throw std::invalid_argument("loudness gain request is outside the supported range");
    }

    LoudnessGainAdvice advice{
        .resulting_gain_centibels = request.current_gain_centibels,
        .estimated_integrated_lufs = request.integrated_lufs,
        .estimated_true_peak_dbtp = request.true_peak_dbtp,
    };
    if (request.integrated_lufs <= kSilenceFloorLufs) {
        return advice;
    }

    const float target_delta_db = request.target_lufs - request.integrated_lufs;
    const float peak_delta_db = request.true_peak_ceiling_dbtp - request.true_peak_dbtp;
    const float transparent_delta_db = std::min(target_delta_db, peak_delta_db);
    const int quantized_delta_centibels =
        static_cast<int>(std::floor(
            transparent_delta_db * 100.0F / static_cast<float>(kGainStepCentibels) + 1.0e-4F
        ))
        * kGainStepCentibels;

    const int minimum_delta =
        static_cast<int>(kMinimumGainCentibels) - request.current_gain_centibels;
    const int maximum_delta =
        static_cast<int>(kMaximumGainCentibels) - request.current_gain_centibels;
    const int bounded_delta = std::clamp(quantized_delta_centibels, minimum_delta, maximum_delta);

    advice.available = true;
    advice.peak_constrained = peak_delta_db < target_delta_db - 0.05F;
    advice.gain_range_constrained = bounded_delta != quantized_delta_centibels;
    advice.gain_delta_centibels = static_cast<std::int16_t>(bounded_delta);
    advice.resulting_gain_centibels =
        static_cast<std::int16_t>(static_cast<int>(request.current_gain_centibels) + bounded_delta);
    const float applied_delta_db = static_cast<float>(bounded_delta) / 100.0F;
    advice.estimated_integrated_lufs = request.integrated_lufs + applied_delta_db;
    advice.estimated_true_peak_dbtp = request.true_peak_dbtp + applied_delta_db;
    advice.target_reached =
        std::abs(advice.estimated_integrated_lufs - request.target_lufs) <= kReachedToleranceLufs;
    return advice;
}

} // namespace echo::audio
