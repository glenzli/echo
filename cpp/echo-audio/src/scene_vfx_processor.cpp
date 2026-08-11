#include "echo/audio/scene_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <optional>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr double kPi = 3.14159265358979323846;
constexpr std::uint32_t kTransitionMillis = 30;
constexpr std::uint32_t kMaximumDelayMillis = 24;
constexpr std::uint32_t kMaximumSampleRate = 384000;
constexpr std::size_t kFilterCount = 4;
constexpr std::size_t kMaximumChannelCount = 2;

bool valid_character(SceneVfxCharacter character) {
    switch (character) {
    case SceneVfxCharacter::Telephone:
    case SceneVfxCharacter::Radio:
    case SceneVfxCharacter::Intercom:
    case SceneVfxCharacter::BehindWall:
    case SceneVfxCharacter::Underwater:
        return true;
    }
    return false;
}

bool same(SceneVfxAdjustment left, SceneVfxAdjustment right) {
    return left.character == right.character && left.enabled == right.enabled
           && left.mix_percent == right.mix_percent
           && left.intensity_percent == right.intensity_percent;
}

float lerp(float start, float end, float progress) {
    return start + progress * (end - start);
}

class Biquad {
  public:
    void identity() {
        b0_ = 1.0F;
        b1_ = 0.0F;
        b2_ = 0.0F;
        a1_ = 0.0F;
        a2_ = 0.0F;
        reset();
    }

    void low_pass(float frequency_hertz, float quality, std::uint32_t sample_rate) {
        const double frequency = bounded_frequency(frequency_hertz, sample_rate);
        const double omega = 2.0 * kPi * frequency / static_cast<double>(sample_rate);
        const double cosine = std::cos(omega);
        const double alpha = std::sin(omega) / (2.0 * static_cast<double>(quality));
        normalized(
            (1.0 - cosine) * 0.5,
            1.0 - cosine,
            (1.0 - cosine) * 0.5,
            1.0 + alpha,
            -2.0 * cosine,
            1.0 - alpha
        );
    }

    void high_pass(float frequency_hertz, float quality, std::uint32_t sample_rate) {
        const double frequency = bounded_frequency(frequency_hertz, sample_rate);
        const double omega = 2.0 * kPi * frequency / static_cast<double>(sample_rate);
        const double cosine = std::cos(omega);
        const double alpha = std::sin(omega) / (2.0 * static_cast<double>(quality));
        normalized(
            (1.0 + cosine) * 0.5,
            -(1.0 + cosine),
            (1.0 + cosine) * 0.5,
            1.0 + alpha,
            -2.0 * cosine,
            1.0 - alpha
        );
    }

    void
    peak(float frequency_hertz, float quality, float gain_decibels, std::uint32_t sample_rate) {
        const double frequency = bounded_frequency(frequency_hertz, sample_rate);
        const double omega = 2.0 * kPi * frequency / static_cast<double>(sample_rate);
        const double cosine = std::cos(omega);
        const double alpha = std::sin(omega) / (2.0 * static_cast<double>(quality));
        const double amplitude = std::pow(10.0, static_cast<double>(gain_decibels) / 40.0);
        normalized(
            1.0 + alpha * amplitude,
            -2.0 * cosine,
            1.0 - alpha * amplitude,
            1.0 + alpha / amplitude,
            -2.0 * cosine,
            1.0 - alpha / amplitude
        );
    }

    float process(float sample, std::size_t channel) {
        State& state = states_[channel];
        const float output = b0_ * sample + state.z1;
        state.z1 = b1_ * sample - a1_ * output + state.z2;
        state.z2 = b2_ * sample - a2_ * output;
        return output;
    }

    void reset() {
        states_.fill({});
    }

  private:
    struct State {
        float z1 = 0.0F;
        float z2 = 0.0F;
    };

    static double bounded_frequency(float frequency_hertz, std::uint32_t sample_rate) {
        const double nyquist_guard = static_cast<double>(sample_rate) * 0.45;
        return std::clamp(static_cast<double>(frequency_hertz), 20.0, nyquist_guard);
    }

    void normalized(double b0, double b1, double b2, double a0, double a1, double a2) {
        const double inverse_a0 = 1.0 / a0;
        b0_ = static_cast<float>(b0 * inverse_a0);
        b1_ = static_cast<float>(b1 * inverse_a0);
        b2_ = static_cast<float>(b2 * inverse_a0);
        a1_ = static_cast<float>(a1 * inverse_a0);
        a2_ = static_cast<float>(a2 * inverse_a0);
        reset();
    }

    float b0_ = 1.0F;
    float b1_ = 0.0F;
    float b2_ = 0.0F;
    float a1_ = 0.0F;
    float a2_ = 0.0F;
    std::array<State, kMaximumChannelCount> states_{};
};

class Bank {
  public:
    Bank(std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        delay_line_(maximum_delay_frames() * channel_count, 0.0F) {}

    void configure(SceneVfxAdjustment adjustment) {
        adjustment_ = adjustment;
        intensity_ = static_cast<float>(adjustment.intensity_percent) / 100.0F;
        mix_ = static_cast<float>(adjustment.mix_percent) / 100.0F;
        centered_amount_ = 0.0F;
        saturation_mix_ = 0.0F;
        delay_base_frames_ = 0.0F;
        delay_sweep_frames_ = 0.0F;
        delay_feedback_ = 0.0F;
        lfo_phase_ = 0.0F;
        lfo_step_ = 0.0F;
        for (Biquad& filter : filters_) {
            filter.identity();
        }

        switch (adjustment.character) {
        case SceneVfxCharacter::Telephone:
            configure_telephone();
            break;
        case SceneVfxCharacter::Radio:
            configure_radio();
            break;
        case SceneVfxCharacter::Intercom:
            configure_intercom();
            break;
        case SceneVfxCharacter::BehindWall:
            configure_behind_wall();
            break;
        case SceneVfxCharacter::Underwater:
            configure_underwater();
            break;
        }
        reset_state();
    }

    [[nodiscard]] SceneVfxAdjustment adjustment() const {
        return adjustment_;
    }

    [[nodiscard]] bool bypassed() const {
        return !adjustment_.enabled || adjustment_.mix_percent == 0
               || adjustment_.intensity_percent == 0;
    }

    std::array<float, 2> process(float left, float right) {
        if (bypassed()) {
            return {left, right};
        }

        std::array<float, 2> input{left, right};
        std::array<float, 2> wet{};
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            float output = input[channel];
            for (Biquad& filter : filters_) {
                output = filter.process(output, channel);
            }
            if (saturation_mix_ > 0.0F) {
                const float drive = 1.0F + 2.5F * intensity_;
                const float saturated = std::tanh(drive * output) / drive;
                output += saturation_mix_ * (saturated - output);
            }
            wet[channel] = output;
        }

        if (adjustment_.character == SceneVfxCharacter::Underwater) {
            process_underwater(wet);
        }
        if (channel_count_ == 1) {
            wet[1] = wet[0];
        } else if (centered_amount_ > 0.0F) {
            const float center = 0.5F * (wet[0] + wet[1]);
            wet[0] += centered_amount_ * (center - wet[0]);
            wet[1] += centered_amount_ * (center - wet[1]);
        }

        return {
            left + mix_ * (wet[0] - left),
            right + mix_ * (wet[1] - right),
        };
    }

    void reset_state() {
        for (Biquad& filter : filters_) {
            filter.reset();
        }
        std::fill(delay_line_.begin(), delay_line_.end(), 0.0F);
        delay_cursor_ = 0;
        lfo_phase_ = 0.0F;
    }

  private:
    [[nodiscard]] std::size_t maximum_delay_frames() const {
        return std::max<std::size_t>(
            2,
            static_cast<std::size_t>(sample_rate_) * kMaximumDelayMillis / 1000U + 2
        );
    }

    void configure_telephone() {
        filters_[0].high_pass(lerp(70.0F, 300.0F, intensity_), 0.707F, sample_rate_);
        const float high_cut = lerp(18000.0F, 3400.0F, intensity_);
        filters_[1].low_pass(high_cut, 0.707F, sample_rate_);
        filters_[2].low_pass(high_cut, 0.707F, sample_rate_);
        filters_[3].peak(1800.0F, 0.9F, 4.0F * intensity_, sample_rate_);
        centered_amount_ = 0.9F * intensity_;
        saturation_mix_ = 0.45F * intensity_;
    }

    void configure_radio() {
        filters_[0].high_pass(lerp(60.0F, 140.0F, intensity_), 0.707F, sample_rate_);
        const float high_cut = lerp(16000.0F, 5000.0F, intensity_);
        filters_[1].low_pass(high_cut, 0.707F, sample_rate_);
        filters_[2].peak(2100.0F, 0.8F, 3.5F * intensity_, sample_rate_);
        filters_[3].peak(450.0F, 0.9F, -2.0F * intensity_, sample_rate_);
        centered_amount_ = 0.65F * intensity_;
        saturation_mix_ = 0.3F * intensity_;
    }

    void configure_intercom() {
        filters_[0].high_pass(lerp(80.0F, 250.0F, intensity_), 0.72F, sample_rate_);
        filters_[1].low_pass(lerp(12000.0F, 4500.0F, intensity_), 0.72F, sample_rate_);
        filters_[2].peak(1200.0F, 1.5F, 4.5F * intensity_, sample_rate_);
        filters_[3].peak(2700.0F, 1.8F, 3.5F * intensity_, sample_rate_);
        centered_amount_ = intensity_;
        saturation_mix_ = 0.65F * intensity_;
    }

    void configure_behind_wall() {
        filters_[0].high_pass(lerp(30.0F, 90.0F, intensity_), 0.707F, sample_rate_);
        const float high_cut = lerp(9000.0F, 650.0F, intensity_);
        filters_[1].low_pass(high_cut, 0.64F, sample_rate_);
        filters_[2].low_pass(high_cut, 0.64F, sample_rate_);
        filters_[3].peak(180.0F, 0.8F, -2.5F * intensity_, sample_rate_);
        centered_amount_ = 0.15F * intensity_;
    }

    void configure_underwater() {
        const float high_cut = lerp(6000.0F, 700.0F, intensity_);
        filters_[0].low_pass(high_cut, 0.62F, sample_rate_);
        filters_[1].low_pass(high_cut, 0.62F, sample_rate_);
        filters_[2].peak(180.0F, 0.8F, 2.0F * intensity_, sample_rate_);
        delay_base_frames_ =
            lerp(6.0F, 14.0F, intensity_) * static_cast<float>(sample_rate_) / 1000.0F;
        delay_sweep_frames_ =
            lerp(1.0F, 4.0F, intensity_) * static_cast<float>(sample_rate_) / 1000.0F;
        delay_feedback_ = lerp(0.08F, 0.36F, intensity_);
        const float lfo_hertz = lerp(0.25F, 0.8F, intensity_);
        lfo_step_ = static_cast<float>(2.0 * kPi) * lfo_hertz / static_cast<float>(sample_rate_);
        centered_amount_ = 0.1F * intensity_;
    }

    float delayed_sample(std::size_t channel, float delay_frames) const {
        const std::size_t delay_size = maximum_delay_frames();
        const float bounded = std::clamp(delay_frames, 1.0F, static_cast<float>(delay_size - 2));
        const std::size_t whole = static_cast<std::size_t>(bounded);
        const float fraction = bounded - static_cast<float>(whole);
        const std::size_t first = (delay_cursor_ + delay_size - whole) % delay_size;
        const std::size_t second = (first + delay_size - 1) % delay_size;
        const float earlier = delay_line_[first * channel_count_ + channel];
        const float later = delay_line_[second * channel_count_ + channel];
        return earlier + fraction * (later - earlier);
    }

    void process_underwater(std::array<float, 2>& wet) {
        const std::size_t delay_size = maximum_delay_frames();
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const float phase = lfo_phase_ + (channel == 0 ? 0.0F : static_cast<float>(kPi * 0.5));
            const float delay = delay_base_frames_ + delay_sweep_frames_ * std::sin(phase);
            const float delayed = delayed_sample(channel, delay);
            delay_line_[delay_cursor_ * channel_count_ + channel] = wet[channel];
            wet[channel] = (wet[channel] + delay_feedback_ * delayed) / (1.0F + delay_feedback_);
        }
        delay_cursor_ = (delay_cursor_ + 1) % delay_size;
        lfo_phase_ += lfo_step_;
        if (lfo_phase_ >= static_cast<float>(2.0 * kPi)) {
            lfo_phase_ -= static_cast<float>(2.0 * kPi);
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    SceneVfxAdjustment adjustment_{};
    float intensity_ = 0.0F;
    float mix_ = 0.0F;
    float centered_amount_ = 0.0F;
    float saturation_mix_ = 0.0F;
    std::array<Biquad, kFilterCount> filters_{};
    std::vector<float> delay_line_;
    std::size_t delay_cursor_ = 0;
    float delay_base_frames_ = 0.0F;
    float delay_sweep_frames_ = 0.0F;
    float delay_feedback_ = 0.0F;
    float lfo_phase_ = 0.0F;
    float lfo_step_ = 0.0F;
};

} // namespace

class SceneVfxProcessor::Impl {
  public:
    Impl(SceneVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        transition_total_frames_(
            std::max<std::size_t>(
                1,
                static_cast<std::size_t>(sample_rate) * kTransitionMillis / 1000U
            )
        ),
        banks_{{Bank(sample_rate, channel_count), Bank(sample_rate, channel_count)}} {
        banks_[0].configure(adjustment);
        banks_[1].configure(adjustment);
    }

    void update(SceneVfxAdjustment adjustment) {
        SceneVfxProcessor::validate(adjustment, sample_rate_, channel_count_);
        if (transitioning_) {
            if (same(banks_[next_index()].adjustment(), adjustment)) {
                pending_.reset();
            } else {
                pending_ = adjustment;
            }
            return;
        }
        if (!same(banks_[active_index_].adjustment(), adjustment)) {
            begin_transition(adjustment);
        }
    }

    void reset() {
        SceneVfxAdjustment desired = banks_[active_index_].adjustment();
        if (transitioning_) {
            desired = pending_.value_or(banks_[next_index()].adjustment());
        }
        active_index_ = 0;
        banks_[0].configure(desired);
        banks_[1].configure(desired);
        transition_frame_ = 0;
        transitioning_ = false;
        pending_.reset();
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("scene VFX channel layout changed");
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t index = frame * channel_count_;
            const float left = samples[index];
            const float right = channel_count_ == 1 ? left : samples[index + 1];
            const auto current = banks_[active_index_].process(left, right);
            std::array<float, 2> output = current;
            if (transitioning_) {
                const auto next = banks_[next_index()].process(left, right);
                const float progress = std::min(
                    1.0F,
                    static_cast<float>(transition_frame_ + 1)
                        / static_cast<float>(transition_total_frames_)
                );
                output[0] += progress * (next[0] - output[0]);
                output[1] += progress * (next[1] - output[1]);
                ++transition_frame_;
                if (transition_frame_ >= transition_total_frames_) {
                    complete_transition();
                }
            }
            samples[index] = output[0];
            if (channel_count_ == 2) {
                samples[index + 1] = output[1];
            }
        }
    }

    [[nodiscard]] bool is_bypassed() const {
        return !transitioning_ && !pending_.has_value() && banks_[active_index_].bypassed();
    }

  private:
    [[nodiscard]] std::size_t next_index() const {
        return 1 - active_index_;
    }

    void begin_transition(SceneVfxAdjustment adjustment) {
        banks_[next_index()].configure(adjustment);
        transition_frame_ = 0;
        transitioning_ = true;
    }

    void complete_transition() {
        active_index_ = next_index();
        transition_frame_ = 0;
        transitioning_ = false;
        if (pending_.has_value()) {
            const SceneVfxAdjustment pending = *pending_;
            pending_.reset();
            if (!same(banks_[active_index_].adjustment(), pending)) {
                begin_transition(pending);
            }
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t transition_total_frames_ = 1;
    std::array<Bank, 2> banks_;
    std::size_t active_index_ = 0;
    std::size_t transition_frame_ = 0;
    bool transitioning_ = false;
    std::optional<SceneVfxAdjustment> pending_;
};

SceneVfxProcessor::SceneVfxProcessor(
    SceneVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    validate(adjustment, sample_rate, channel_count);
    impl_ = std::make_unique<Impl>(adjustment, sample_rate, channel_count);
}

SceneVfxProcessor::~SceneVfxProcessor() = default;

void SceneVfxProcessor::validate(
    SceneVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (!valid_character(adjustment.character) || sample_rate < 8000
        || sample_rate > kMaximumSampleRate || (channel_count != 1 && channel_count != 2)
        || adjustment.mix_percent > 100 || adjustment.intensity_percent > 100) {
        throw std::invalid_argument("scene VFX parameters are outside the supported range");
    }
}

void SceneVfxProcessor::update(SceneVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void SceneVfxProcessor::reset() {
    impl_->reset();
}

void SceneVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

std::size_t SceneVfxProcessor::latency_frames() const {
    return 0;
}

bool SceneVfxProcessor::is_bypassed() const {
    return impl_->is_bypassed();
}

} // namespace echo::audio
