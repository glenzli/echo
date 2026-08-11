#include "echo/audio/transform_vfx_processor.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 384000;
constexpr std::uint32_t kLatencyMillis = 50;
constexpr std::uint32_t kParameterSmoothingMillis = 20;
constexpr std::uint32_t kCharacterTransitionMillis = 30;

float finite(float value) {
    return std::isfinite(value) ? value : 0.0F;
}

float wrap_phase(float phase) {
    phase -= std::floor(phase);
    return phase;
}

float semitone_ratio(float semitones) {
    return std::exp2(semitones / 12.0F);
}

} // namespace

TransformVfxProcessor::TransformVfxProcessor(
    TransformVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : sample_rate_(sample_rate), channel_count_(channel_count), target_(adjustment) {
    validate(adjustment, sample_rate, channel_count);
    latency_frames_ =
        std::max<std::size_t>(1, static_cast<std::size_t>(sample_rate_) * kLatencyMillis / 1000);
    pitch_range_frames_ = std::max<std::size_t>(1, latency_frames_ / 2);
    buffer_frames_ = latency_frames_ + pitch_range_frames_ + kHilbertRadius + 4;
    delay_buffer_.assign(buffer_frames_ * channel_count_, 0.0F);
    giant_lowpass_state_.assign(channel_count_, 0.0F);

    const float smoothing_frames = std::max(
        1.0F,
        static_cast<float>(sample_rate_) * static_cast<float>(kParameterSmoothingMillis) / 1000.0F
    );
    smoothing_coefficient_ = 1.0F - std::exp(-1.0F / smoothing_frames);
    character_transition_frames_ = std::max<std::size_t>(
        1,
        static_cast<std::size_t>(sample_rate_) * kCharacterTransitionMillis / 1000
    );

    for (int tap = -static_cast<int>(kHilbertRadius); tap <= static_cast<int>(kHilbertRadius);
         ++tap) {
        float coefficient = 0.0F;
        if (tap != 0 && std::abs(tap) % 2 == 1) {
            const float ideal = 2.0F / (std::numbers::pi_v<float> * static_cast<float>(tap));
            const float position = static_cast<float>(tap + static_cast<int>(kHilbertRadius))
                                   / static_cast<float>(kHilbertRadius * 2);
            const float window =
                0.54F - 0.46F * std::cos(2.0F * std::numbers::pi_v<float> * position);
            coefficient = ideal * window;
        }
        hilbert_coefficients_[static_cast<std::size_t>(tap + static_cast<int>(kHilbertRadius))] =
            coefficient;
    }

    reset();
}

void TransformVfxProcessor::update(TransformVfxAdjustment adjustment) {
    validate(adjustment, sample_rate_, channel_count_);
    if (adjustment.character != to_character_) {
        from_character_ = to_character_;
        to_character_ = adjustment.character;
        character_transition_remaining_ = character_transition_frames_;
        const std::size_t index = character_index(to_character_);
        pitch_phase_[index] = 0.0F;
        oscillator_phase_[index] = 0.0F;
        if (to_character_ == TransformVfxCharacter::Giant) {
            std::fill(giant_lowpass_state_.begin(), giant_lowpass_state_.end(), 0.0F);
        }
    }
    target_ = adjustment;
    target_mix_ = adjustment.enabled ? static_cast<float>(adjustment.mix_percent) / 100.0F : 0.0F;
    target_amount_ = static_cast<float>(adjustment.amount_percent) / 100.0F;
}

void TransformVfxProcessor::reset() {
    std::fill(delay_buffer_.begin(), delay_buffer_.end(), 0.0F);
    std::fill(giant_lowpass_state_.begin(), giant_lowpass_state_.end(), 0.0F);
    pitch_phase_.fill(0.0F);
    oscillator_phase_.fill(0.0F);
    write_cursor_ = 0;
    from_character_ = target_.character;
    to_character_ = target_.character;
    character_transition_remaining_ = 0;
    target_mix_ = target_.enabled ? static_cast<float>(target_.mix_percent) / 100.0F : 0.0F;
    current_mix_ = target_mix_;
    target_amount_ = static_cast<float>(target_.amount_percent) / 100.0F;
    current_amount_ = target_amount_;
}

void TransformVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
        throw std::invalid_argument("transform VFX channel layout changed");
    }

    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        current_mix_ += smoothing_coefficient_ * (target_mix_ - current_mix_);
        current_amount_ += smoothing_coefficient_ * (target_amount_ - current_amount_);

        float character_mix = 1.0F;
        if (character_transition_remaining_ > 0) {
            character_mix = 1.0F
                            - static_cast<float>(character_transition_remaining_)
                                  / static_cast<float>(character_transition_frames_);
        }

        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const std::size_t sample_index = frame * channel_count_ + channel;
            const float input = finite(samples[sample_index]);
            const float dry = delayed_sample(channel, latency_frames_);
            const float from = character_sample(from_character_, channel, current_amount_);
            float wet = from;
            if (from_character_ != to_character_) {
                const float to = character_sample(to_character_, channel, current_amount_);
                wet = from + character_mix * (to - from);
            }
            samples[sample_index] = finite(dry + current_mix_ * (wet - dry));
            delay_buffer_[write_cursor_ * channel_count_ + channel] = input;
        }

        advance_character(from_character_, current_amount_);
        if (from_character_ != to_character_) {
            advance_character(to_character_, current_amount_);
        }
        write_cursor_ = (write_cursor_ + 1) % buffer_frames_;

        if (character_transition_remaining_ > 0) {
            --character_transition_remaining_;
            if (character_transition_remaining_ == 0) {
                from_character_ = to_character_;
            }
        }
    }
}

std::size_t TransformVfxProcessor::latency_frames() const noexcept {
    return latency_frames_;
}

TransformVfxAdjustment TransformVfxProcessor::adjustment() const noexcept {
    return target_;
}

void TransformVfxProcessor::validate(
    TransformVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate
        || (channel_count != 1 && channel_count != 2) || adjustment.mix_percent > 100
        || adjustment.amount_percent > 100
        || static_cast<std::uint8_t>(adjustment.character)
               > static_cast<std::uint8_t>(TransformVfxCharacter::Ghost)) {
        throw std::invalid_argument("transform VFX parameters are outside the supported range");
    }
}

std::size_t TransformVfxProcessor::character_index(TransformVfxCharacter character) {
    return static_cast<std::size_t>(character);
}

float TransformVfxProcessor::read_delay(std::size_t channel, float delay_frames) const {
    const float bounded = std::clamp(delay_frames, 1.0F, static_cast<float>(buffer_frames_ - 2));
    const std::size_t whole = static_cast<std::size_t>(bounded);
    const float fraction = bounded - static_cast<float>(whole);
    const std::size_t newer = (write_cursor_ + buffer_frames_ - whole) % buffer_frames_;
    const std::size_t older = (newer + buffer_frames_ - 1) % buffer_frames_;
    const float newer_sample = delay_buffer_[newer * channel_count_ + channel];
    const float older_sample = delay_buffer_[older * channel_count_ + channel];
    return newer_sample + fraction * (older_sample - newer_sample);
}

float TransformVfxProcessor::delayed_sample(std::size_t channel, std::size_t delay_frames) const {
    const std::size_t bounded = std::min(delay_frames, buffer_frames_ - 1);
    const std::size_t index = (write_cursor_ + buffer_frames_ - bounded) % buffer_frames_;
    return delay_buffer_[index * channel_count_ + channel];
}

float TransformVfxProcessor::pitch_sample(
    TransformVfxCharacter character,
    std::size_t channel,
    float amount
) {
    float semitones = 0.0F;
    switch (character) {
    case TransformVfxCharacter::Monster:
        semitones = -(4.0F + 8.0F * amount);
        break;
    case TransformVfxCharacter::Tiny:
        semitones = 4.0F + 8.0F * amount;
        break;
    case TransformVfxCharacter::Giant:
        semitones = -(2.0F + 5.0F * amount);
        break;
    case TransformVfxCharacter::Robot:
    case TransformVfxCharacter::Ghost:
        return delayed_sample(channel, latency_frames_);
    }

    const float ratio = semitone_ratio(semitones);
    const float phase = pitch_phase_[character_index(character)];
    const float second_phase = wrap_phase(phase + 0.5F);
    const auto delay_for = [&](float head_phase) {
        const float sweep = ratio >= 1.0F ? 1.0F - head_phase : head_phase;
        return static_cast<float>(latency_frames_)
               + sweep * static_cast<float>(pitch_range_frames_);
    };
    const float first = read_delay(channel, delay_for(phase));
    const float second = read_delay(channel, delay_for(second_phase));
    const float first_gain = std::sin(std::numbers::pi_v<float> * phase);
    const float second_gain = std::sin(std::numbers::pi_v<float> * second_phase);
    const float pitched =
        (first_gain * first + second_gain * second) / std::max(first_gain + second_gain, 1.0F);

    if (character == TransformVfxCharacter::Monster) {
        const float oscillator = std::sin(
            2.0F * std::numbers::pi_v<float> * oscillator_phase_[character_index(character)]
        );
        return pitched * (0.86F + 0.14F * oscillator);
    }
    if (character == TransformVfxCharacter::Giant) {
        const float cutoff = 2200.0F - 900.0F * amount;
        const float alpha =
            1.0F
            - std::exp(
                -2.0F * std::numbers::pi_v<float> * cutoff / static_cast<float>(sample_rate_)
            );
        giant_lowpass_state_[channel] += alpha * (pitched - giant_lowpass_state_[channel]);
        return giant_lowpass_state_[channel];
    }
    return pitched;
}

float TransformVfxProcessor::robot_sample(std::size_t channel, float amount) {
    const float dry = delayed_sample(channel, latency_frames_);
    const float oscillator = std::sin(
        2.0F * std::numbers::pi_v<float>
        * oscillator_phase_[character_index(TransformVfxCharacter::Robot)]
    );
    const float metallic = dry * oscillator;
    const float carrier_mix = 0.65F + 0.35F * amount;
    return dry + carrier_mix * (metallic - dry);
}

float TransformVfxProcessor::ghost_sample(std::size_t channel, float amount) {
    const std::size_t center_delay = latency_frames_ + kHilbertRadius;
    const float in_phase = delayed_sample(channel, center_delay);
    float quadrature = 0.0F;
    for (int tap = -static_cast<int>(kHilbertRadius); tap <= static_cast<int>(kHilbertRadius);
         ++tap) {
        const std::size_t delay = static_cast<std::size_t>(static_cast<int>(center_delay) + tap);
        quadrature +=
            hilbert_coefficients_[static_cast<std::size_t>(tap + static_cast<int>(kHilbertRadius))]
            * delayed_sample(channel, delay);
    }
    const float phase = oscillator_phase_[character_index(TransformVfxCharacter::Ghost)];
    const float angle = 2.0F * std::numbers::pi_v<float> * phase;
    const float direction = channel == 1 ? -1.0F : 1.0F;
    const float shifted = in_phase * std::cos(angle) - direction * quadrature * std::sin(angle);
    const float shift_mix = 0.6F + 0.4F * amount;
    return in_phase + shift_mix * (shifted - in_phase);
}

float TransformVfxProcessor::character_sample(
    TransformVfxCharacter character,
    std::size_t channel,
    float amount
) {
    switch (character) {
    case TransformVfxCharacter::Robot:
        return robot_sample(channel, amount);
    case TransformVfxCharacter::Monster:
    case TransformVfxCharacter::Tiny:
    case TransformVfxCharacter::Giant:
        return pitch_sample(character, channel, amount);
    case TransformVfxCharacter::Ghost:
        return ghost_sample(channel, amount);
    }
    return delayed_sample(channel, latency_frames_);
}

void TransformVfxProcessor::advance_character(TransformVfxCharacter character, float amount) {
    const std::size_t index = character_index(character);
    switch (character) {
    case TransformVfxCharacter::Robot: {
        const float frequency = 45.0F + 155.0F * amount;
        oscillator_phase_[index] =
            wrap_phase(oscillator_phase_[index] + frequency / static_cast<float>(sample_rate_));
        break;
    }
    case TransformVfxCharacter::Monster: {
        const float ratio = semitone_ratio(-(4.0F + 8.0F * amount));
        pitch_phase_[index] = wrap_phase(
            pitch_phase_[index] + (1.0F - ratio) / static_cast<float>(pitch_range_frames_)
        );
        const float frequency = 22.0F + 30.0F * amount;
        oscillator_phase_[index] =
            wrap_phase(oscillator_phase_[index] + frequency / static_cast<float>(sample_rate_));
        break;
    }
    case TransformVfxCharacter::Tiny: {
        const float ratio = semitone_ratio(4.0F + 8.0F * amount);
        pitch_phase_[index] = wrap_phase(
            pitch_phase_[index] + (ratio - 1.0F) / static_cast<float>(pitch_range_frames_)
        );
        break;
    }
    case TransformVfxCharacter::Giant: {
        const float ratio = semitone_ratio(-(2.0F + 5.0F * amount));
        pitch_phase_[index] = wrap_phase(
            pitch_phase_[index] + (1.0F - ratio) / static_cast<float>(pitch_range_frames_)
        );
        break;
    }
    case TransformVfxCharacter::Ghost: {
        const float frequency = 4.0F + 26.0F * amount;
        oscillator_phase_[index] =
            wrap_phase(oscillator_phase_[index] + frequency / static_cast<float>(sample_rate_));
        break;
    }
    }
}

} // namespace echo::audio
