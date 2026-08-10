#include "echo/audio/algorithmic_reverb.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <stdexcept>
#include <utility>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::array<double, 4> kDelayMilliseconds{29.7, 37.1, 41.1, 43.7};
constexpr std::array<double, 4> kEarlyMilliseconds{5.0, 11.0, 17.0, 23.0};

std::size_t frames_for_millis(double millis, std::uint32_t sample_rate) {
    return std::max<std::size_t>(
        1,
        static_cast<std::size_t>(std::round(millis * static_cast<double>(sample_rate) / 1000.0))
    );
}

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

} // namespace

struct AlgorithmicReverb::Engine {
    struct DelayLine {
        std::vector<float> samples;
        std::size_t cursor = 0;
        float damped = 0.0F;

        [[nodiscard]] float read() const {
            return samples[cursor];
        }

        void write(float sample) {
            samples[cursor] = finite(sample);
            cursor = (cursor + 1) % samples.size();
        }

        void reset() {
            std::fill(samples.begin(), samples.end(), 0.0F);
            cursor = 0;
            damped = 0.0F;
        }
    };

    ReverbAdjustment adjustment;
    std::uint32_t sample_rate;
    std::vector<float> pre_left;
    std::vector<float> pre_right;
    std::size_t pre_cursor = 0;
    std::size_t pre_delay_frames = 0;
    std::array<std::size_t, 4> early_frames{};
    std::array<DelayLine, 4> delays;
    std::array<float, 4> feedback{};
    float damping_alpha = 1.0F;
    float wet_lowpass_alpha = 1.0F;
    float wet_highpass_decay = 0.0F;
    std::array<float, 2> lowpass_state{};
    std::array<float, 2> highpass_state{};
    std::array<float, 2> highpass_input{};

    Engine(ReverbAdjustment authored, std::uint32_t rate) :
        adjustment(authored), sample_rate(rate) {
        pre_delay_frames = static_cast<std::size_t>(std::round(
            static_cast<double>(adjustment.pre_delay_millis) * static_cast<double>(sample_rate)
            / 1000.0
        ));
        const std::size_t maximum_early = frames_for_millis(
            static_cast<double>(adjustment.pre_delay_millis) + kEarlyMilliseconds.back(),
            sample_rate
        );
        pre_left.assign(maximum_early + 1, 0.0F);
        pre_right.assign(maximum_early + 1, 0.0F);
        for (std::size_t index = 0; index < early_frames.size(); ++index) {
            early_frames[index] = frames_for_millis(
                static_cast<double>(adjustment.pre_delay_millis) + kEarlyMilliseconds[index],
                sample_rate
            );
        }

        const double size_scale = 0.65 + 0.007 * static_cast<double>(adjustment.size_percent);
        const double decay_seconds = static_cast<double>(adjustment.decay_millis) / 1000.0;
        for (std::size_t index = 0; index < delays.size(); ++index) {
            const std::size_t length =
                frames_for_millis(kDelayMilliseconds[index] * size_scale, sample_rate);
            delays[index].samples.assign(length, 0.0F);
            const double seconds = static_cast<double>(length) / static_cast<double>(sample_rate);
            feedback[index] = static_cast<float>(std::pow(0.001, seconds / decay_seconds));
        }

        const double damping_cutoff =
            16000.0 - 13500.0 * static_cast<double>(adjustment.damping_percent) / 100.0;
        damping_alpha = static_cast<float>(
            1.0 - std::exp(-2.0 * std::numbers::pi * damping_cutoff / sample_rate)
        );
        const double wet_high_cut = std::min(
            static_cast<double>(adjustment.high_cut_hertz),
            0.45 * static_cast<double>(sample_rate)
        );
        wet_lowpass_alpha = static_cast<float>(
            1.0 - std::exp(-2.0 * std::numbers::pi * wet_high_cut / sample_rate)
        );
        wet_highpass_decay = static_cast<float>(std::exp(
            -2.0 * std::numbers::pi * static_cast<double>(adjustment.low_cut_hertz) / sample_rate
        ));
    }

    [[nodiscard]] float delayed(const std::vector<float>& buffer, std::size_t frames) const {
        return buffer[(pre_cursor + buffer.size() - frames % buffer.size()) % buffer.size()];
    }

    [[nodiscard]] float filter_wet(float sample, std::size_t channel) {
        const float highpassed =
            wet_highpass_decay * (highpass_state[channel] + sample - highpass_input[channel]);
        highpass_input[channel] = sample;
        highpass_state[channel] = highpassed;
        lowpass_state[channel] += wet_lowpass_alpha * (highpassed - lowpass_state[channel]);
        return lowpass_state[channel];
    }

    [[nodiscard]] std::array<float, 2> process(float left, float right) {
        left = finite(left);
        right = finite(right);
        pre_left[pre_cursor] = left;
        pre_right[pre_cursor] = right;
        const float pre_l = delayed(pre_left, pre_delay_frames);
        const float pre_r = delayed(pre_right, pre_delay_frames);

        float early_l = 0.0F;
        float early_r = 0.0F;
        constexpr std::array<float, 4> early_gain{0.42F, 0.31F, 0.23F, 0.17F};
        for (std::size_t index = 0; index < early_frames.size(); ++index) {
            early_l += delayed(pre_left, early_frames[index]) * early_gain[index];
            early_r += delayed(pre_right, early_frames[index])
                       * early_gain[early_frames.size() - 1 - index];
        }
        pre_cursor = (pre_cursor + 1) % pre_left.size();

        std::array<float, 4> tail{};
        float sum = 0.0F;
        for (std::size_t index = 0; index < delays.size(); ++index) {
            tail[index] = delays[index].read();
            sum += tail[index];
        }
        const std::array<float, 4> injection{
            pre_l,
            pre_r,
            0.7071F * (pre_l + pre_r),
            0.7071F * (pre_l - pre_r),
        };
        for (std::size_t index = 0; index < delays.size(); ++index) {
            const float mixed = tail[index] - 0.5F * sum;
            delays[index].damped += damping_alpha * (mixed - delays[index].damped);
            delays[index].write(0.30F * injection[index] + feedback[index] * delays[index].damped);
        }

        float wet_l =
            0.34F * early_l + 0.32F * (tail[0] + tail[2] + 0.7F * tail[3] - 0.4F * tail[1]);
        float wet_r =
            0.34F * early_r + 0.32F * (tail[1] + tail[3] + 0.7F * tail[2] - 0.4F * tail[0]);
        wet_l = filter_wet(wet_l, 0);
        wet_r = filter_wet(wet_r, 1);
        const float wet =
            adjustment.enabled ? static_cast<float>(adjustment.mix_percent) / 100.0F : 0.0F;
        const float dry = 1.0F - wet;
        return {finite(dry * left + wet * wet_l), finite(dry * right + wet * wet_r)};
    }

    void reset() {
        std::fill(pre_left.begin(), pre_left.end(), 0.0F);
        std::fill(pre_right.begin(), pre_right.end(), 0.0F);
        pre_cursor = 0;
        for (auto& delay : delays) {
            delay.reset();
        }
        lowpass_state = {};
        highpass_state = {};
        highpass_input = {};
    }
};

AlgorithmicReverb::AlgorithmicReverb(
    ReverbAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : sample_rate_(sample_rate), channel_count_(channel_count) {
    validate(adjustment, sample_rate, channel_count);
    transition_frames_ = std::max<std::size_t>(1, sample_rate / 20U);
    active_ = std::make_unique<Engine>(adjustment, sample_rate);
}

AlgorithmicReverb::~AlgorithmicReverb() = default;

void AlgorithmicReverb::validate(
    ReverbAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (sample_rate < 8000 || channel_count == 0 || channel_count > 2
        || adjustment.mix_percent > 100 || adjustment.pre_delay_millis > 200
        || adjustment.decay_millis < 100 || adjustment.decay_millis > 12000
        || adjustment.size_percent < 10 || adjustment.size_percent > 100
        || adjustment.damping_percent > 100 || adjustment.low_cut_hertz < 20
        || adjustment.low_cut_hertz > 1000 || adjustment.high_cut_hertz < 1000
        || adjustment.high_cut_hertz > 20000
        || adjustment.low_cut_hertz >= adjustment.high_cut_hertz) {
        throw std::invalid_argument("reverb parameters are outside the supported range");
    }
}

bool AlgorithmicReverb::same(ReverbAdjustment left, ReverbAdjustment right) {
    return left.enabled == right.enabled && left.mix_percent == right.mix_percent
           && left.pre_delay_millis == right.pre_delay_millis
           && left.decay_millis == right.decay_millis && left.size_percent == right.size_percent
           && left.damping_percent == right.damping_percent
           && left.low_cut_hertz == right.low_cut_hertz
           && left.high_cut_hertz == right.high_cut_hertz;
}

void AlgorithmicReverb::begin_transition(ReverbAdjustment adjustment) {
    next_ = std::make_unique<Engine>(adjustment, sample_rate_);
    transition_frame_ = 0;
}

void AlgorithmicReverb::update(ReverbAdjustment adjustment) {
    validate(adjustment, sample_rate_, channel_count_);
    if (next_ != nullptr) {
        pending_ = adjustment;
    } else if (!same(active_->adjustment, adjustment)) {
        begin_transition(adjustment);
    }
}

void AlgorithmicReverb::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || channel_count != channel_count_) {
        throw std::invalid_argument("reverb channel layout changed");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        float& left = samples[frame * channel_count];
        float& right = samples[frame * channel_count + (channel_count == 1 ? 0 : 1)];
        const float input_left = left;
        const float input_right = right;
        const auto current = active_->process(input_left, input_right);
        std::array<float, 2> output = current;
        if (next_ != nullptr) {
            const auto next = next_->process(input_left, input_right);
            const float progress = std::min(
                1.0F,
                static_cast<float>(transition_frame_) / static_cast<float>(transition_frames_)
            );
            output[0] = current[0] + progress * (next[0] - current[0]);
            output[1] = current[1] + progress * (next[1] - current[1]);
            ++transition_frame_;
            if (transition_frame_ >= transition_frames_) {
                active_ = std::move(next_);
                if (pending_.has_value() && !same(active_->adjustment, *pending_)) {
                    const ReverbAdjustment pending = *pending_;
                    pending_.reset();
                    begin_transition(pending);
                } else {
                    pending_.reset();
                }
            }
        }
        left = output[0];
        if (channel_count > 1) {
            right = output[1];
        }
    }
}

void AlgorithmicReverb::reset() {
    active_->reset();
    next_.reset();
    pending_.reset();
    transition_frame_ = 0;
}

bool AlgorithmicReverb::is_bypassed() const {
    return !active_->adjustment.enabled || active_->adjustment.mix_percent == 0;
}

} // namespace echo::audio
