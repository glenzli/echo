#include "echo/audio/de_esser.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>
#include <stdexcept>

namespace echo::audio {
namespace {

float time_coefficient(float milliseconds, std::uint32_t sample_rate) {
    return std::exp(-1.0F / (0.001F * milliseconds * static_cast<float>(sample_rate)));
}

float amplitude_from_centibels(float centibels) {
    return std::pow(10.0F, centibels / 2000.0F);
}

void validate(DeEsserAdjustment adjustment) {
    if (adjustment.frequency_hertz < 3000 || adjustment.frequency_hertz > 12000
        || adjustment.threshold_centibels < -6000 || adjustment.threshold_centibels > 0
        || adjustment.reduction_centibels > 1800) {
        throw std::invalid_argument("de-esser parameters are outside the supported range");
    }
}

} // namespace

DeEsser::DeEsser(
    DeEsserAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) :
    sample_rate_(sample_rate), channel_count_(channel_count), target_(adjustment),
    lowpass_state_(channel_count, 0.0F), high_components_(channel_count, 0.0F),
    frequency_hertz_(adjustment.frequency_hertz),
    threshold_centibels_(adjustment.threshold_centibels),
    reduction_centibels_(adjustment.enabled ? adjustment.reduction_centibels : 0.0F) {
    if (sample_rate == 0 || channel_count == 0) {
        throw std::invalid_argument("de-esser requires valid audio dimensions");
    }
    validate(adjustment);
}

void DeEsser::update(DeEsserAdjustment adjustment) {
    validate(adjustment);
    target_ = adjustment;
}

void DeEsser::reset() {
    std::fill(lowpass_state_.begin(), lowpass_state_.end(), 0.0F);
    envelope_ = 0.0F;
    gain_ = 1.0F;
}

void DeEsser::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || channel_count != channel_count_) {
        throw std::invalid_argument("de-esser channel layout changed");
    }
    const float parameter_coefficient = time_coefficient(45.0F, sample_rate_);
    const float detector_attack = time_coefficient(1.5F, sample_rate_);
    const float detector_release = time_coefficient(65.0F, sample_rate_);
    const float gain_attack = time_coefficient(2.0F, sample_rate_);
    const float gain_release = time_coefficient(90.0F, sample_rate_);

    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        frequency_hertz_ =
            parameter_coefficient * frequency_hertz_
            + (1.0F - parameter_coefficient) * static_cast<float>(target_.frequency_hertz);
        threshold_centibels_ =
            parameter_coefficient * threshold_centibels_
            + (1.0F - parameter_coefficient) * static_cast<float>(target_.threshold_centibels);
        const float target_reduction =
            target_.enabled ? static_cast<float>(target_.reduction_centibels) : 0.0F;
        reduction_centibels_ = parameter_coefficient * reduction_centibels_
                               + (1.0F - parameter_coefficient) * target_reduction;

        const float lowpass_coefficient = std::exp(
            -2.0F * std::numbers::pi_v<float> * frequency_hertz_ / static_cast<float>(sample_rate_)
        );
        float linked_level = 0.0F;
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t index = frame * channel_count_ + channel;
            lowpass_state_[channel] = (1.0F - lowpass_coefficient) * samples[index]
                                      + lowpass_coefficient * lowpass_state_[channel];
            high_components_[channel] = samples[index] - lowpass_state_[channel];
            linked_level = std::max(linked_level, std::abs(high_components_[channel]));
        }
        const float detector_coefficient =
            linked_level > envelope_ ? detector_attack : detector_release;
        envelope_ = detector_coefficient * envelope_ + (1.0F - detector_coefficient) * linked_level;

        const float threshold = amplitude_from_centibels(threshold_centibels_);
        const float over = std::max(
            0.0F,
            20.0F * std::log10(std::max(envelope_, 0.000001F) / std::max(threshold, 0.000001F))
        );
        const float desired_reduction = std::min(reduction_centibels_ / 100.0F, over * 0.75F);
        const float target_gain = std::pow(10.0F, -desired_reduction / 20.0F);
        const float gain_coefficient = target_gain < gain_ ? gain_attack : gain_release;
        gain_ = gain_coefficient * gain_ + (1.0F - gain_coefficient) * target_gain;

        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t index = frame * channel_count_ + channel;
            samples[index] = lowpass_state_[channel] + high_components_[channel] * gain_;
        }
    }
}

float DeEsser::attenuation_decibels() const {
    return -20.0F * std::log10(std::max(gain_, 0.000001F));
}

} // namespace echo::audio
