#include "echo/audio/de_plosive_processor.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 384000;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

float smoothstep(float value) {
    const float bounded = std::clamp(value, 0.0F, 1.0F);
    return bounded * bounded * (3.0F - 2.0F * bounded);
}

} // namespace

DePlosiveProcessor::DePlosiveProcessor(
    DePlosiveAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) :
    sample_rate_(sample_rate), channel_count_(channel_count), target_(adjustment),
    lowpass_state_(channel_count, 0.0F), low_components_(channel_count, 0.0F) {
    validate(adjustment, sample_rate, channel_count);
    parameter_step_ = coefficient(20.0F);
    refresh_targets();
    lowpass_alpha_ = target_lowpass_alpha_;
    sensitivity_ = target_sensitivity_;
    minimum_gain_ = target_minimum_gain_;
    release_step_ = target_release_step_;
}

void DePlosiveProcessor::update(DePlosiveAdjustment adjustment) {
    validate(adjustment, sample_rate_, channel_count_);
    target_ = adjustment;
    refresh_targets();
}

void DePlosiveProcessor::reset() {
    std::fill(lowpass_state_.begin(), lowpass_state_.end(), 0.0F);
    std::fill(low_components_.begin(), low_components_.end(), 0.0F);
    fast_low_envelope_ = 0.0F;
    slow_low_envelope_ = 0.0F;
    broadband_envelope_ = 0.0F;
    gain_ = 1.0F;
}

void DePlosiveProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
        throw std::invalid_argument("de-plosive channel layout changed");
    }

    const float fast_attack = coefficient(0.7F);
    const float fast_release = coefficient(28.0F);
    const float slow_attack = coefficient(85.0F);
    const float slow_release = coefficient(420.0F);
    const float broadband_attack = coefficient(1.5F);
    const float broadband_release = coefficient(55.0F);
    const float gain_attack = coefficient(0.8F);

    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        lowpass_alpha_ += parameter_step_ * (target_lowpass_alpha_ - lowpass_alpha_);
        sensitivity_ += parameter_step_ * (target_sensitivity_ - sensitivity_);
        minimum_gain_ += parameter_step_ * (target_minimum_gain_ - minimum_gain_);
        release_step_ += parameter_step_ * (target_release_step_ - release_step_);

        float linked_low = 0.0F;
        float linked_full = 0.0F;
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t index = frame * channel_count_ + channel;
            const float sample = finite(samples[index]);
            lowpass_state_[channel] += lowpass_alpha_ * (sample - lowpass_state_[channel]);
            lowpass_state_[channel] = finite(lowpass_state_[channel]);
            low_components_[channel] = lowpass_state_[channel];
            linked_low = std::max(linked_low, std::abs(lowpass_state_[channel]));
            linked_full = std::max(linked_full, std::abs(sample));
        }

        const float fast_step = linked_low > fast_low_envelope_ ? fast_attack : fast_release;
        fast_low_envelope_ += fast_step * (linked_low - fast_low_envelope_);
        const float slow_step = linked_low > slow_low_envelope_ ? slow_attack : slow_release;
        slow_low_envelope_ += slow_step * (linked_low - slow_low_envelope_);
        const float full_step =
            linked_full > broadband_envelope_ ? broadband_attack : broadband_release;
        broadband_envelope_ += full_step * (linked_full - broadband_envelope_);

        const float prominence = fast_low_envelope_ / std::max(slow_low_envelope_, 0.0002F);
        const float dominance = fast_low_envelope_ / std::max(broadband_envelope_, 0.0002F);
        const float onset_threshold = 3.6F - 2.0F * sensitivity_;
        const float dominance_threshold = 0.82F - 0.32F * sensitivity_;
        const float onset =
            smoothstep((prominence - onset_threshold) / std::max(0.5F, onset_threshold * 0.9F));
        const float low_dominance = smoothstep(
            (dominance - dominance_threshold) / std::max(0.1F, 1.0F - dominance_threshold)
        );
        const float desired_gain = 1.0F - onset * low_dominance * (1.0F - minimum_gain_);
        const float gain_step = desired_gain < gain_ ? gain_attack : release_step_;
        gain_ += gain_step * (desired_gain - gain_);

        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t index = frame * channel_count_ + channel;
            const float sample = finite(samples[index]);
            samples[index] = finite(sample + (gain_ - 1.0F) * low_components_[channel]);
        }
    }
}

DePlosiveAdjustment DePlosiveProcessor::adjustment() const {
    return target_;
}

float DePlosiveProcessor::attenuation_decibels() const {
    return std::max(0.0F, -20.0F * std::log10(std::max(gain_, 1.0E-6F)));
}

void DePlosiveProcessor::validate(
    DePlosiveAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate || channel_count == 0
        || adjustment.frequency_hertz < 80 || adjustment.frequency_hertz > 240
        || adjustment.sensitivity_percent > 100 || adjustment.reduction_centibels > 1800
        || adjustment.release_millis < 40 || adjustment.release_millis > 500) {
        throw std::invalid_argument("de-plosive parameters are outside the supported range");
    }
}

float DePlosiveProcessor::coefficient(float milliseconds) const {
    const float frames = milliseconds * static_cast<float>(sample_rate_) / 1000.0F;
    return 1.0F - std::exp(-1.0F / std::max(frames, 1.0F));
}

float DePlosiveProcessor::lowpass_alpha(std::uint16_t frequency_hertz) const {
    return 1.0F
           - std::exp(
               -2.0F * std::numbers::pi_v<float>
               * static_cast<float>(frequency_hertz) / static_cast<float>(sample_rate_)
           );
}

void DePlosiveProcessor::refresh_targets() {
    target_lowpass_alpha_ = lowpass_alpha(target_.frequency_hertz);
    target_sensitivity_ = static_cast<float>(target_.sensitivity_percent) / 100.0F;
    target_minimum_gain_ =
        target_.enabled
            ? std::pow(10.0F, -static_cast<float>(target_.reduction_centibels) / 2000.0F)
            : 1.0F;
    target_release_step_ = coefficient(static_cast<float>(target_.release_millis));
}

} // namespace echo::audio
