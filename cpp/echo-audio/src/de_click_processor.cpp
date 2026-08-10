#include "echo/audio/de_click_processor.hpp"

#include <algorithm>
#include <cmath>
#include <limits>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 384000;
constexpr std::uint16_t kMinimumClickMicroseconds = 50;
constexpr std::uint16_t kMaximumClickMicroseconds = 2000;
constexpr float kParameterSmoothingMilliseconds = 15.0F;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

std::size_t frames_for_microseconds(std::uint16_t microseconds, std::uint32_t sample_rate) {
    const std::uint64_t numerator =
        static_cast<std::uint64_t>(microseconds) * static_cast<std::uint64_t>(sample_rate);
    return std::max<std::size_t>(1, static_cast<std::size_t>((numerator + 999999U) / 1000000U));
}

} // namespace

DeClickProcessor::DeClickProcessor(
    DeClickParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : sample_rate_(sample_rate), channel_count_(channel_count), target_(parameters) {
    validate(parameters, sample_rate, channel_count);
    maximum_supported_click_frames_ =
        frames_for_microseconds(kMaximumClickMicroseconds, sample_rate_);
    latency_frames_ = maximum_supported_click_frames_ + 1;
    history_capacity_frames_ = latency_frames_ + 3;
    history_.assign(history_capacity_frames_ * channel_count_, 0.0F);
    channel_states_.resize(channel_count_);
    sensitivity_percent_ = static_cast<float>(parameters.sensitivity_percent);
    repair_mix_ =
        parameters.enabled ? static_cast<float>(parameters.repair_percent) / 100.0F : 0.0F;
    const float smoothing_frames =
        kParameterSmoothingMilliseconds * static_cast<float>(sample_rate_) / 1000.0F;
    parameter_coefficient_ = 1.0F - std::exp(-1.0F / std::max(1.0F, smoothing_frames));
}

void DeClickProcessor::update(DeClickParameters parameters) {
    validate(parameters, sample_rate_, channel_count_);
    target_ = parameters;
}

void DeClickProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
        throw std::invalid_argument("de-click channel layout changed");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t input_index = frame * channel_count_ + channel;
            history_[history_cursor_ * channel_count_ + channel] = finite(samples[input_index]);
        }

        sensitivity_percent_ +=
            parameter_coefficient_
            * (static_cast<float>(target_.sensitivity_percent) - sensitivity_percent_);
        const float target_mix =
            target_.enabled ? static_cast<float>(target_.repair_percent) / 100.0F : 0.0F;
        repair_mix_ += parameter_coefficient_ * (target_mix - repair_mix_);

        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t output_index = frame * channel_count_ + channel;
            if (frames_seen_ < latency_frames_) {
                samples[output_index] = 0.0F;
                continue;
            }
            const float delayed_input = history(channel, latency_frames_);
            const float repair = repaired_sample(channel, delayed_input);
            samples[output_index] = finite(delayed_input + repair_mix_ * (repair - delayed_input));
        }

        history_cursor_ = (history_cursor_ + 1) % history_capacity_frames_;
        if (frames_seen_ != std::numeric_limits<std::size_t>::max()) {
            ++frames_seen_;
        }
    }
}

void DeClickProcessor::reset() {
    std::fill(history_.begin(), history_.end(), 0.0F);
    std::fill(channel_states_.begin(), channel_states_.end(), ChannelState{});
    history_cursor_ = 0;
    frames_seen_ = 0;
    sensitivity_percent_ = static_cast<float>(target_.sensitivity_percent);
    repair_mix_ = target_.enabled ? static_cast<float>(target_.repair_percent) / 100.0F : 0.0F;
}

DeClickParameters DeClickProcessor::parameters() const {
    return target_;
}

std::size_t DeClickProcessor::latency_frames() const {
    return latency_frames_;
}

bool DeClickProcessor::is_bypassed() const {
    return !target_.enabled && repair_mix_ < 1.0E-5F;
}

void DeClickProcessor::validate(
    DeClickParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate || channel_count == 0
        || channel_count > 8 || parameters.sensitivity_percent > 100
        || parameters.maximum_click_microseconds < kMinimumClickMicroseconds
        || parameters.maximum_click_microseconds > kMaximumClickMicroseconds
        || parameters.repair_percent > 100) {
        throw std::invalid_argument("de-click parameters are outside the supported range");
    }
}

float DeClickProcessor::history(std::size_t channel, std::size_t frames_ago) const {
    const std::size_t frame =
        (history_cursor_ + history_capacity_frames_ - frames_ago % history_capacity_frames_)
        % history_capacity_frames_;
    return history_[frame * channel_count_ + channel];
}

std::size_t DeClickProcessor::maximum_click_frames() const {
    return std::min(
        maximum_supported_click_frames_,
        frames_for_microseconds(target_.maximum_click_microseconds, sample_rate_)
    );
}

std::size_t DeClickProcessor::detect_click(std::size_t channel) const {
    const float candidate = history(channel, latency_frames_);
    const float before = history(channel, latency_frames_ + 1);
    const float before_two = history(channel, latency_frames_ + 2);
    const float preceding_slope = before - before_two;
    const float start_jump = candidate - before;
    const float sensitivity = std::clamp(sensitivity_percent_ / 100.0F, 0.0F, 1.0F);
    const float absolute_floor = 0.00035F + (1.0F - sensitivity) * 0.003F;
    const float slope_multiplier = 18.0F - sensitivity * 12.0F;
    const float initial_threshold = absolute_floor + slope_multiplier * std::abs(preceding_slope);
    if (std::abs(start_jump) <= initial_threshold) {
        return 0;
    }

    for (std::size_t length = 1; length <= maximum_click_frames(); ++length) {
        const float last = history(channel, latency_frames_ - (length - 1));
        const float after = history(channel, latency_frames_ - length);
        const float after_two = history(channel, latency_frames_ - (length + 1));
        const float ending_jump = after - last;
        const float following_slope = after_two - after;
        const float outside_slope = std::max(std::abs(preceding_slope), std::abs(following_slope));
        const float threshold = absolute_floor + slope_multiplier * outside_slope;
        if (std::abs(start_jump) <= threshold || std::abs(ending_jump) <= threshold * 0.5F
            || start_jump * ending_jump >= 0.0F) {
            continue;
        }

        const float expected_after =
            before + 0.5F * (preceding_slope + following_slope) * static_cast<float>(length + 1);
        const float return_tolerance = std::max(threshold * 1.5F, std::abs(start_jump) * 0.22F);
        if (std::abs(after - expected_after) > return_tolerance) {
            continue;
        }

        float maximum_residual = 0.0F;
        float residual_sum = 0.0F;
        for (std::size_t offset = 0; offset < length; ++offset) {
            const float progress = static_cast<float>(offset + 1) / static_cast<float>(length + 1);
            const float prediction = before + progress * (after - before);
            const float residual =
                std::abs(history(channel, latency_frames_ - offset) - prediction);
            maximum_residual = std::max(maximum_residual, residual);
            residual_sum += residual;
        }
        const float mean_residual = residual_sum / static_cast<float>(length);
        if (maximum_residual > threshold && mean_residual > threshold * 0.35F) {
            return length;
        }
    }
    return 0;
}

float DeClickProcessor::repaired_sample(std::size_t channel, float delayed_input) {
    ChannelState& state = channel_states_[channel];
    if (state.repair_frame == 0) {
        const std::size_t length = detect_click(channel);
        if (length == 0) {
            return delayed_input;
        }
        state.repair_total_frames = length;
        state.repair_frame = 1;
        state.repair_start = history(channel, latency_frames_ + 1);
        state.repair_end = history(channel, latency_frames_ - length);
    }

    const float progress =
        static_cast<float>(state.repair_frame) / static_cast<float>(state.repair_total_frames + 1);
    const float repair = state.repair_start + progress * (state.repair_end - state.repair_start);
    if (++state.repair_frame > state.repair_total_frames) {
        state = {};
    }
    return repair;
}

} // namespace echo::audio
