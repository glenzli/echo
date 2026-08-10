#include "echo/audio/dynamics_processor.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr float kSoftKneeDecibels = 6.0F;
constexpr float kMinimumPeak = 1.0E-9F;

} // namespace

DynamicsProcessor::DynamicsProcessor(CompressorAdjustment adjustment, std::uint32_t sample_rate) :
    adjustment_(adjustment), sample_rate_(sample_rate) {
    if (sample_rate_ == 0) {
        throw std::invalid_argument("compressor sample rate must be positive");
    }
    validate(adjustment_);
    refresh_coefficients();
}

void DynamicsProcessor::update(CompressorAdjustment adjustment) {
    validate(adjustment);
    adjustment_ = adjustment;
    refresh_coefficients();
}

void DynamicsProcessor::reset() {
    gain_ = 1.0F;
}

void DynamicsProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || channel_count == 0) {
        throw std::invalid_argument("compressor requires valid audio dimensions");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        float peak = 0.0F;
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            peak = std::max(peak, std::abs(samples[frame * channel_count + channel]));
        }
        const float target = adjustment_.enabled ? desired_gain(peak) : 1.0F;
        const float coefficient = target < gain_ ? attack_coefficient_ : release_coefficient_;
        gain_ += (target - gain_) * coefficient;
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            samples[frame * channel_count + channel] *= gain_;
        }
    }
}

CompressorAdjustment DynamicsProcessor::adjustment() const {
    return adjustment_;
}

float DynamicsProcessor::current_gain() const {
    return gain_;
}

void DynamicsProcessor::validate(CompressorAdjustment adjustment) const {
    if (adjustment.threshold_centibels < -6000 || adjustment.threshold_centibels > 0
        || adjustment.ratio_tenths < 10 || adjustment.ratio_tenths > 200
        || adjustment.attack_millis < 1 || adjustment.attack_millis > 200
        || adjustment.release_millis < 20 || adjustment.release_millis > 2000
        || adjustment.makeup_centibels < 0 || adjustment.makeup_centibels > 2400) {
        throw std::invalid_argument("compressor parameters are outside the supported range");
    }
}

void DynamicsProcessor::refresh_coefficients() {
    const auto smoothing_coefficient = [this](std::uint16_t millis) {
        const float frames =
            static_cast<float>(sample_rate_) * static_cast<float>(millis) / 1000.0F;
        return 1.0F - std::exp(-1.0F / std::max(frames, 1.0F));
    };
    attack_coefficient_ = smoothing_coefficient(adjustment_.attack_millis);
    release_coefficient_ = smoothing_coefficient(adjustment_.release_millis);
}

float DynamicsProcessor::desired_gain(float peak) const {
    const float input_decibels = 20.0F * std::log10(std::max(peak, kMinimumPeak));
    const float threshold = static_cast<float>(adjustment_.threshold_centibels) / 100.0F;
    const float ratio = static_cast<float>(adjustment_.ratio_tenths) / 10.0F;
    const float offset = input_decibels - threshold;
    float output_decibels = input_decibels;
    if (offset > kSoftKneeDecibels / 2.0F) {
        output_decibels = threshold + offset / ratio;
    } else if (offset > -kSoftKneeDecibels / 2.0F) {
        const float knee_progress = offset + kSoftKneeDecibels / 2.0F;
        output_decibels +=
            (1.0F / ratio - 1.0F) * knee_progress * knee_progress / (2.0F * kSoftKneeDecibels);
    }
    const float makeup = static_cast<float>(adjustment_.makeup_centibels) / 100.0F;
    return std::pow(10.0F, (output_decibels - input_decibels + makeup) / 20.0F);
}

} // namespace echo::audio
