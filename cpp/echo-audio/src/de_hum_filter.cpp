#include "echo/audio/de_hum_filter.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kTransitionMilliseconds = 30;
constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 384000;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

} // namespace

float DeHumFilter::Section::process(float sample, std::size_t channel) {
    State& state = states[channel];
    const float notched = finite(b0 * sample + state.z1);
    state.z1 = finite(b1 * sample - a1 * notched + state.z2);
    state.z2 = finite(b2 * sample - a2 * notched);
    if (std::abs(state.z1) < 1.0E-20F) {
        state.z1 = 0.0F;
    }
    if (std::abs(state.z2) < 1.0E-20F) {
        state.z2 = 0.0F;
    }
    return finite(sample + mix * (notched - sample));
}

void DeHumFilter::Section::reset() {
    states = {};
}

float DeHumFilter::Bank::process(float sample, std::size_t channel) {
    float output = finite(sample);
    if (bypassed) {
        return output;
    }
    for (std::size_t index = 0; index < section_count; ++index) {
        output = sections[index].process(output, channel);
    }
    return output;
}

void DeHumFilter::Bank::reset() {
    for (std::size_t index = 0; index < section_count; ++index) {
        sections[index].reset();
    }
}

DeHumFilter::DeHumFilter(
    DeHumParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : sample_rate_(sample_rate), channel_count_(channel_count) {
    validate(parameters, sample_rate, channel_count);
    transition_total_frames_ = std::max<std::size_t>(
        1,
        static_cast<std::size_t>(sample_rate) * kTransitionMilliseconds / 1000
    );
    current_ = prepare(parameters, sample_rate);
}

void DeHumFilter::update(DeHumParameters parameters) {
    validate(parameters, sample_rate_, channel_count_);
    if (!next_.has_value()) {
        if (!same(current_.parameters, parameters)) {
            next_ = prepare(parameters, sample_rate_);
            transition_frame_ = 0;
        }
        return;
    }
    if (same(next_->parameters, parameters)) {
        pending_.reset();
    } else {
        pending_ = prepare(parameters, sample_rate_);
    }
}

void DeHumFilter::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
        throw std::invalid_argument("de-hum channel layout changed");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float progress = next_.has_value()
                                   ? std::min(
                                         1.0F,
                                         static_cast<float>(transition_frame_ + 1)
                                             / static_cast<float>(transition_total_frames_)
                                     )
                                   : 0.0F;
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t index = frame * channel_count_ + channel;
            const float current_output = current_.process(samples[index], channel);
            if (next_.has_value()) {
                const float next_output = next_->process(samples[index], channel);
                samples[index] = finite(current_output + progress * (next_output - current_output));
            } else {
                samples[index] = current_output;
            }
        }
        if (next_.has_value() && ++transition_frame_ >= transition_total_frames_) {
            current_ = std::move(*next_);
            next_.reset();
            transition_frame_ = 0;
            if (pending_.has_value() && !same(current_.parameters, pending_->parameters)) {
                next_ = std::move(*pending_);
                pending_.reset();
            } else {
                pending_.reset();
            }
        }
    }
}

void DeHumFilter::reset() {
    if (pending_.has_value()) {
        current_ = std::move(*pending_);
    } else if (next_.has_value()) {
        current_ = std::move(*next_);
    }
    next_.reset();
    pending_.reset();
    transition_frame_ = 0;
    current_.reset();
}

DeHumParameters DeHumFilter::parameters() const {
    if (pending_.has_value()) {
        return pending_->parameters;
    }
    return next_.has_value() ? next_->parameters : current_.parameters;
}

bool DeHumFilter::is_bypassed() const {
    return current_.bypassed && !next_.has_value() && !pending_.has_value();
}

void DeHumFilter::validate(
    DeHumParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate || channel_count == 0
        || channel_count > kMaximumChannels
        || (parameters.fundamental_hertz != 50 && parameters.fundamental_hertz != 60)
        || parameters.harmonic_count == 0 || parameters.harmonic_count > kMaximumHarmonics
        || parameters.quality_tenths < 50 || parameters.quality_tenths > 1000
        || parameters.depth_centibels > 4800) {
        throw std::invalid_argument("de-hum parameters are outside the supported range");
    }
}

DeHumFilter::Bank DeHumFilter::prepare(DeHumParameters parameters, std::uint32_t sample_rate) {
    Bank bank;
    bank.parameters = parameters;
    bank.bypassed = !parameters.enabled || parameters.depth_centibels == 0;
    if (bank.bypassed) {
        return bank;
    }

    const double quality = static_cast<double>(parameters.quality_tenths) / 10.0;
    const double center_amplitude =
        std::pow(10.0, -static_cast<double>(parameters.depth_centibels) / 2000.0);
    const double maximum_frequency = 0.45 * static_cast<double>(sample_rate);
    for (std::size_t harmonic = 1; harmonic <= parameters.harmonic_count; ++harmonic) {
        const double frequency =
            static_cast<double>(parameters.fundamental_hertz) * static_cast<double>(harmonic);
        if (frequency >= maximum_frequency) {
            break;
        }
        const double omega = 2.0 * std::numbers::pi * frequency / static_cast<double>(sample_rate);
        const double alpha = std::sin(omega) / (2.0 * quality);
        const double cosine = std::cos(omega);
        const double a0 = 1.0 + alpha;

        Section& section = bank.sections[bank.section_count++];
        section.b0 = static_cast<float>(1.0 / a0);
        section.b1 = static_cast<float>((-2.0 * cosine) / a0);
        section.b2 = static_cast<float>(1.0 / a0);
        section.a1 = static_cast<float>((-2.0 * cosine) / a0);
        section.a2 = static_cast<float>((1.0 - alpha) / a0);
        section.mix = static_cast<float>(1.0 - center_amplitude);
    }
    bank.bypassed = bank.section_count == 0;
    return bank;
}

bool DeHumFilter::same(DeHumParameters left, DeHumParameters right) {
    return left.enabled == right.enabled && left.fundamental_hertz == right.fundamental_hertz
           && left.harmonic_count == right.harmonic_count
           && left.quality_tenths == right.quality_tenths
           && left.depth_centibels == right.depth_centibels;
}

} // namespace echo::audio
