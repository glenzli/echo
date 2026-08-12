#include "echo/audio/spring_space_reverb.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <optional>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kSupportedSampleRate = 48'000;
constexpr std::size_t kPathCount = 3;
constexpr std::size_t kDispersionSectionCount = 8;
constexpr std::size_t kStateCount = 3;
constexpr std::size_t kMaximumPreDelayFrames = kSupportedSampleRate / 5;
constexpr std::size_t kMaximumLoopFrames = 1'930;
constexpr std::size_t kMixRampFrames = kSupportedSampleRate / 50;
constexpr std::size_t kStructureTransitionFrames = kSupportedSampleRate / 20;

constexpr std::array<std::size_t, kPathCount> kBaseLoopFrames{1'091, 1'303, 1'543};
constexpr std::array<double, kDispersionSectionCount> kSectionFrequencies{
    310.0,
    520.0,
    830.0,
    1'300.0,
    2'050.0,
    3'200.0,
    5'000.0,
    7'800.0,
};
constexpr std::array<double, kDispersionSectionCount> kSectionRadii{
    0.70,
    0.74,
    0.78,
    0.82,
    0.85,
    0.88,
    0.91,
    0.93,
};
constexpr std::array<double, kPathCount> kPathDispersionScale{0.977, 1.0, 1.023};

constexpr float kInverseSqrtTwo = 0.7071067811865475F;
constexpr float kInverseSqrtThree = 0.5773502691896258F;
constexpr float kFeedbackCoupling = 0.18F;
constexpr float kInjectionGain = 0.27F;
constexpr float kOutputGain = 0.30F;

float finite_sample(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

void validate_adjustment(
    ReverbAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (adjustment.character != ReverbCharacter::Spring || sample_rate != kSupportedSampleRate
        || channel_count == 0 || channel_count > 2 || adjustment.mix_percent > 100
        || adjustment.pre_delay_millis > 200 || adjustment.decay_millis < 100
        || adjustment.decay_millis > 12'000 || adjustment.size_percent < 10
        || adjustment.size_percent > 100 || adjustment.damping_percent > 100
        || adjustment.low_cut_hertz < 20 || adjustment.low_cut_hertz > 1'000
        || adjustment.high_cut_hertz < 1'000 || adjustment.high_cut_hertz > 20'000
        || adjustment.low_cut_hertz >= adjustment.high_cut_hertz) {
        throw std::invalid_argument("spring space parameters are outside the supported range");
    }
}

bool same_structure(ReverbAdjustment left, ReverbAdjustment right) {
    return left.character == right.character && left.pre_delay_millis == right.pre_delay_millis
           && left.decay_millis == right.decay_millis && left.size_percent == right.size_percent
           && left.damping_percent == right.damping_percent
           && left.low_cut_hertz == right.low_cut_hertz
           && left.high_cut_hertz == right.high_cut_hertz;
}

float wet_mix(ReverbAdjustment adjustment) {
    return adjustment.enabled ? static_cast<float>(adjustment.mix_percent) / 100.0F : 0.0F;
}

float one_pole_alpha(double cutoff_hertz) {
    return static_cast<float>(
        1.0
        - std::exp(
            -2.0 * std::numbers::pi * cutoff_hertz / static_cast<double>(kSupportedSampleRate)
        )
    );
}

struct AllpassSection {
    float denominator_one = 0.0F;
    float denominator_two = 0.0F;
    float input_one = 0.0F;
    float input_two = 0.0F;
    float output_one = 0.0F;
    float output_two = 0.0F;

    void configure(double frequency_hertz, double radius) {
        const double angle =
            2.0 * std::numbers::pi * frequency_hertz / static_cast<double>(kSupportedSampleRate);
        denominator_one = static_cast<float>(-2.0 * radius * std::cos(angle));
        denominator_two = static_cast<float>(radius * radius);
        reset();
    }

    [[nodiscard]] float process(float input) {
        input = finite_sample(input);
        const float output = finite_sample(
            denominator_two * input + denominator_one * input_one + input_two
            - denominator_one * output_one - denominator_two * output_two
        );
        input_two = input_one;
        input_one = input;
        output_two = output_one;
        output_one = output;
        return output;
    }

    void reset() {
        input_one = 0.0F;
        input_two = 0.0F;
        output_one = 0.0F;
        output_two = 0.0F;
    }
};

struct SpringState {
    ReverbAdjustment adjustment{};
    std::array<float, kMaximumPreDelayFrames + 1> pre_left{};
    std::array<float, kMaximumPreDelayFrames + 1> pre_right{};
    std::size_t pre_delay_length = 1;
    std::size_t pre_cursor = 0;
    std::array<std::array<float, kMaximumLoopFrames>, kPathCount> loop_samples{};
    std::array<std::size_t, kPathCount> loop_lengths{};
    std::array<std::size_t, kPathCount> loop_cursors{};
    std::array<std::array<AllpassSection, kDispersionSectionCount>, kPathCount> dispersion{};
    std::array<float, kPathCount> feedback{};
    std::array<float, kPathCount> damping_state{};
    float damping_alpha = 1.0F;
    float wet_lowpass_alpha = 1.0F;
    float wet_highpass_decay = 0.0F;
    std::array<float, 2> lowpass_state{};
    std::array<float, 2> highpass_state{};
    std::array<float, 2> highpass_input{};

    void configure(ReverbAdjustment authored) {
        adjustment = authored;
        pre_delay_length = static_cast<std::size_t>(adjustment.pre_delay_millis) * 48U + 1U;

        const double size_scale = 0.75 + 0.005 * static_cast<double>(adjustment.size_percent);
        const double decay_seconds = static_cast<double>(adjustment.decay_millis) / 1'000.0;
        const double radius_offset = 0.0006 * (static_cast<double>(adjustment.size_percent) - 55.0);
        for (std::size_t path = 0; path < kPathCount; ++path) {
            loop_lengths[path] = std::clamp<std::size_t>(
                static_cast<std::size_t>(
                    std::round(static_cast<double>(kBaseLoopFrames[path]) * size_scale)
                ),
                1,
                kMaximumLoopFrames
            );
            const double effective_loop_frames =
                static_cast<double>(loop_lengths[path] + 2U * kDispersionSectionCount);
            feedback[path] = static_cast<float>(std::pow(
                0.001,
                effective_loop_frames / (decay_seconds * static_cast<double>(kSupportedSampleRate))
            ));
            for (std::size_t section = 0; section < kDispersionSectionCount; ++section) {
                const double radius =
                    std::clamp(kSectionRadii[section] + radius_offset, 0.62, 0.965);
                const double frequency =
                    kSectionFrequencies[section] * kPathDispersionScale[path]
                    * (0.92 + 0.0015 * static_cast<double>(adjustment.size_percent));
                dispersion[path][section].configure(frequency, radius);
            }
        }

        const double damping_cutoff =
            14'500.0 - 12'000.0 * static_cast<double>(adjustment.damping_percent) / 100.0;
        damping_alpha = one_pole_alpha(damping_cutoff);
        wet_lowpass_alpha = one_pole_alpha(std::min<double>(adjustment.high_cut_hertz, 21'600.0));
        wet_highpass_decay = static_cast<float>(std::exp(
            -2.0 * std::numbers::pi * static_cast<double>(adjustment.low_cut_hertz)
            / static_cast<double>(kSupportedSampleRate)
        ));
        reset();
    }

    void reset() {
        pre_left.fill(0.0F);
        pre_right.fill(0.0F);
        pre_cursor = 0;
        for (auto& path : loop_samples) {
            path.fill(0.0F);
        }
        loop_cursors.fill(0);
        damping_state.fill(0.0F);
        for (auto& path : dispersion) {
            for (auto& section : path) {
                section.reset();
            }
        }
        lowpass_state.fill(0.0F);
        highpass_state.fill(0.0F);
        highpass_input.fill(0.0F);
    }

    [[nodiscard]] std::array<float, 2> process_wet(float left, float right) {
        left = finite_sample(left);
        right = finite_sample(right);
        pre_left[pre_cursor] = left;
        pre_right[pre_cursor] = right;
        const std::size_t read_cursor = (pre_cursor + 1U) % pre_delay_length;
        const float delayed_left = pre_left[read_cursor];
        const float delayed_right = pre_right[read_cursor];
        pre_cursor = read_cursor;

        const float mid = kInverseSqrtTwo * (delayed_left + delayed_right);
        const float side = kInverseSqrtTwo * (delayed_left - delayed_right);
        const std::array<float, kPathCount> injection{
            kInverseSqrtThree * mid + kInverseSqrtTwo * side,
            kInverseSqrtThree * mid - kInverseSqrtTwo * side,
            kInverseSqrtThree * mid,
        };

        std::array<float, kPathCount> dispersed{};
        for (std::size_t path = 0; path < kPathCount; ++path) {
            float value = loop_samples[path][loop_cursors[path]];
            for (auto& section : dispersion[path]) {
                value = section.process(value);
            }
            dispersed[path] = value;
        }

        const float sum = dispersed[0] + dispersed[1] + dispersed[2];
        for (std::size_t path = 0; path < kPathCount; ++path) {
            const float householder = dispersed[path] - (2.0F / 3.0F) * sum;
            const float scattered =
                dispersed[path] + kFeedbackCoupling * (householder - dispersed[path]);
            damping_state[path] += damping_alpha * (scattered - damping_state[path]);
            loop_samples[path][loop_cursors[path]] = finite_sample(
                kInjectionGain * injection[path] + feedback[path] * damping_state[path]
            );
            loop_cursors[path] = (loop_cursors[path] + 1U) % loop_lengths[path];
        }

        const float output_mid = kInverseSqrtThree * (dispersed[0] + dispersed[1] + dispersed[2]);
        const float output_side = kInverseSqrtTwo * (dispersed[0] - dispersed[1]);
        return {
            filter_wet(kOutputGain * kInverseSqrtTwo * (output_mid + output_side), 0),
            filter_wet(kOutputGain * kInverseSqrtTwo * (output_mid - output_side), 1),
        };
    }

  private:
    [[nodiscard]] float filter_wet(float sample, std::size_t channel) {
        const float highpassed =
            wet_highpass_decay * (highpass_state[channel] + sample - highpass_input[channel]);
        highpass_input[channel] = sample;
        highpass_state[channel] = finite_sample(highpassed);
        lowpass_state[channel] +=
            wet_lowpass_alpha * (highpass_state[channel] - lowpass_state[channel]);
        lowpass_state[channel] = finite_sample(lowpass_state[channel]);
        return lowpass_state[channel];
    }
};

} // namespace

class SpringSpaceReverb::Impl {
  public:
    Impl(ReverbAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), latest_(adjustment) {
        validate_adjustment(adjustment, sample_rate_, channel_count_);
        states_[active_index_].configure(adjustment);
        mix_current_ = wet_mix(adjustment);
        mix_target_ = mix_current_;
    }

    void update(ReverbAdjustment adjustment) {
        validate_adjustment(adjustment, sample_rate_, channel_count_);
        latest_ = adjustment;
        begin_mix_ramp(wet_mix(adjustment));

        if (next_index_.has_value()) {
            if (same_structure(states_[*next_index_].adjustment, adjustment)) {
                pending_index_.reset();
                return;
            }
            const std::size_t slot = pending_index_.has_value() ? *pending_index_ : free_slot();
            states_[slot].configure(adjustment);
            pending_index_ = slot;
            return;
        }
        if (!same_structure(states_[active_index_].adjustment, adjustment)) {
            const std::size_t slot = free_slot();
            states_[slot].configure(adjustment);
            next_index_ = slot;
            transition_frame_ = 0;
        }
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("spring space channel layout changed");
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t index = frame * channel_count_;
            const float input_left = finite_sample(samples[index]);
            const float input_right =
                channel_count_ == 1 ? input_left : finite_sample(samples[index + 1]);
            const auto output = process_frame(input_left, input_right);
            const float output_left = output[0];
            const float output_right = output[1];
            samples[index] =
                channel_count_ == 1 ? 0.5F * (output_left + output_right) : output_left;
            if (channel_count_ == 2) {
                samples[index + 1] = output_right;
            }
        }
    }

    [[nodiscard]] std::array<float, 2> process_frame(float left, float right) {
        left = finite_sample(left);
        right = finite_sample(right);
        const auto current = states_[active_index_].process_wet(left, right);
        std::array<float, 2> wet = current;
        if (next_index_.has_value()) {
            const auto next = states_[*next_index_].process_wet(left, right);
            const float progress = std::min(
                1.0F,
                static_cast<float>(transition_frame_)
                    / static_cast<float>(kStructureTransitionFrames)
            );
            wet[0] = current[0] + progress * (next[0] - current[0]);
            wet[1] = current[1] + progress * (next[1] - current[1]);
            ++transition_frame_;
            if (transition_frame_ >= kStructureTransitionFrames) {
                finish_transition();
            }
        }

        advance_mix();
        const float dry_mix = 1.0F - mix_current_;
        const std::array<float, 2> output{
            finite_sample(dry_mix * left + mix_current_ * wet[0]),
            finite_sample(dry_mix * right + mix_current_ * wet[1]),
        };
        if (channel_count_ == 1) {
            const float mono = 0.5F * (output[0] + output[1]);
            return {mono, mono};
        }
        return output;
    }

    void reset() {
        if (pending_index_.has_value()) {
            active_index_ = *pending_index_;
        } else if (next_index_.has_value()) {
            active_index_ = *next_index_;
        }
        for (auto& state : states_) {
            state.reset();
        }
        next_index_.reset();
        pending_index_.reset();
        transition_frame_ = 0;
        mix_current_ = wet_mix(latest_);
        mix_target_ = mix_current_;
        mix_step_ = 0.0F;
        mix_frames_remaining_ = 0;
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !latest_.enabled || latest_.mix_percent == 0;
    }

  private:
    [[nodiscard]] std::size_t free_slot() const {
        for (std::size_t slot = 0; slot < kStateCount; ++slot) {
            if (slot != active_index_ && (!next_index_.has_value() || slot != *next_index_)
                && (!pending_index_.has_value() || slot != *pending_index_)) {
                return slot;
            }
        }
        throw std::logic_error("spring space has no preallocated transition slot");
    }

    void begin_mix_ramp(float target) {
        mix_target_ = target;
        mix_frames_remaining_ = kMixRampFrames;
        mix_step_ = (mix_target_ - mix_current_) / static_cast<float>(kMixRampFrames);
    }

    void advance_mix() {
        if (mix_frames_remaining_ == 0) {
            return;
        }
        mix_current_ += mix_step_;
        --mix_frames_remaining_;
        if (mix_frames_remaining_ == 0) {
            mix_current_ = mix_target_;
            mix_step_ = 0.0F;
        }
    }

    void finish_transition() {
        active_index_ = *next_index_;
        next_index_.reset();
        transition_frame_ = 0;
        if (pending_index_.has_value()) {
            if (!same_structure(
                    states_[active_index_].adjustment,
                    states_[*pending_index_].adjustment
                )) {
                next_index_ = *pending_index_;
            }
            pending_index_.reset();
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    ReverbAdjustment latest_{};
    std::array<SpringState, kStateCount> states_{};
    std::size_t active_index_ = 0;
    std::optional<std::size_t> next_index_;
    std::optional<std::size_t> pending_index_;
    std::size_t transition_frame_ = 0;
    float mix_current_ = 0.0F;
    float mix_target_ = 0.0F;
    float mix_step_ = 0.0F;
    std::size_t mix_frames_remaining_ = 0;
};

SpringSpaceReverb::SpringSpaceReverb(
    ReverbAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

SpringSpaceReverb::~SpringSpaceReverb() = default;

void SpringSpaceReverb::validate(
    ReverbAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    validate_adjustment(adjustment, sample_rate, channel_count);
}

void SpringSpaceReverb::update(ReverbAdjustment adjustment) {
    impl_->update(adjustment);
}

std::array<float, 2> SpringSpaceReverb::process_frame(float left, float right) {
    return impl_->process_frame(left, right);
}

void SpringSpaceReverb::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void SpringSpaceReverb::reset() {
    impl_->reset();
}

std::size_t SpringSpaceReverb::latency_frames() const noexcept {
    return 0;
}

bool SpringSpaceReverb::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}

} // namespace echo::audio
