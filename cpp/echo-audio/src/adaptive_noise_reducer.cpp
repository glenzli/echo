#include "echo/audio/adaptive_noise_reducer.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

float time_coefficient(float milliseconds, std::uint32_t sample_rate) {
    return std::exp(-1.0F / (0.001F * milliseconds * static_cast<float>(sample_rate)));
}

float amplitude_from_centibels(float centibels) {
    return std::pow(10.0F, -centibels / 2000.0F);
}

void validate(NoiseReductionAdjustment adjustment) {
    if (adjustment.reduction_centibels > 2400 || adjustment.sensitivity_percent > 100
        || adjustment.smoothing_millis < 20 || adjustment.smoothing_millis > 1000) {
        throw std::invalid_argument("noise reduction parameters are outside the supported range");
    }
}

float smoothstep(float value) {
    const float bounded = std::clamp(value, 0.0F, 1.0F);
    return bounded * bounded * (3.0F - 2.0F * bounded);
}

} // namespace

AdaptiveNoiseReducer::AdaptiveNoiseReducer(
    NoiseReductionAdjustment adjustment,
    std::uint32_t sample_rate
) : sample_rate_(sample_rate), target_(adjustment) {
    if (sample_rate == 0) {
        throw std::invalid_argument("noise reduction sample rate must be positive");
    }
    validate(adjustment);
    reduction_decibels_ = adjustment.enabled ? adjustment.reduction_centibels : 0.0F;
    sensitivity_ = static_cast<float>(adjustment.sensitivity_percent) / 100.0F;
}

void AdaptiveNoiseReducer::update(NoiseReductionAdjustment adjustment) {
    validate(adjustment);
    target_ = adjustment;
}

void AdaptiveNoiseReducer::reset() {
    envelope_ = 0.0F;
    noise_floor_ = 0.0025F;
    gain_ = 1.0F;
}

void AdaptiveNoiseReducer::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || channel_count == 0) {
        throw std::invalid_argument("noise reduction requires valid audio dimensions");
    }
    const float parameter_coefficient = time_coefficient(45.0F, sample_rate_);
    const float envelope_attack = time_coefficient(4.0F, sample_rate_);
    const float envelope_release = time_coefficient(70.0F, sample_rate_);
    const float floor_fall = time_coefficient(120.0F, sample_rate_);
    const float floor_rise = time_coefficient(5000.0F, sample_rate_);
    const float open_coefficient = time_coefficient(8.0F, sample_rate_);

    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float target_reduction =
            target_.enabled ? static_cast<float>(target_.reduction_centibels) : 0.0F;
        reduction_decibels_ = parameter_coefficient * reduction_decibels_
                              + (1.0F - parameter_coefficient) * target_reduction;
        const float target_sensitivity = static_cast<float>(target_.sensitivity_percent) / 100.0F;
        sensitivity_ = parameter_coefficient * sensitivity_
                       + (1.0F - parameter_coefficient) * target_sensitivity;

        float linked_level = 0.0F;
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            linked_level =
                std::max(linked_level, std::abs(samples[frame * channel_count + channel]));
        }
        const float envelope_coefficient =
            linked_level > envelope_ ? envelope_attack : envelope_release;
        envelope_ = envelope_coefficient * envelope_ + (1.0F - envelope_coefficient) * linked_level;

        const float floor_coefficient = envelope_ < noise_floor_ ? floor_fall : floor_rise;
        noise_floor_ = std::clamp(
            floor_coefficient * noise_floor_ + (1.0F - floor_coefficient) * envelope_,
            0.00001F,
            0.08F
        );

        const float threshold = std::max(0.00002F, noise_floor_ * (1.35F + sensitivity_ * 1.65F));
        const float openness = smoothstep((envelope_ / threshold - 0.70F) / 0.90F);
        const float minimum_gain = amplitude_from_centibels(reduction_decibels_);
        const float target_gain = minimum_gain + (1.0F - minimum_gain) * openness;
        const float closing_coefficient =
            time_coefficient(static_cast<float>(target_.smoothing_millis), sample_rate_);
        const float gain_coefficient = target_gain > gain_ ? open_coefficient : closing_coefficient;
        gain_ = gain_coefficient * gain_ + (1.0F - gain_coefficient) * target_gain;

        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            samples[frame * channel_count + channel] *= gain_;
        }
    }
}

float AdaptiveNoiseReducer::attenuation_decibels() const {
    return -20.0F * std::log10(std::max(gain_, 0.000001F));
}

} // namespace echo::audio
