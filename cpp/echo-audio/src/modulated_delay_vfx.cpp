#include "modulated_delay_vfx.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>
#include <vector>

namespace echo::audio::detail {
namespace {

constexpr float kMaximumFeedbackSample = 8.0F;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

float smoothing_coefficient(std::uint32_t sample_rate) {
    return static_cast<float>(1.0 - std::exp(-1.0 / (0.02 * static_cast<double>(sample_rate))));
}

float smooth(float& current, float target, float coefficient) {
    current += coefficient * (target - current);
    return current;
}

class FractionalDelay {
  public:
    FractionalDelay(
        std::uint32_t sample_rate,
        std::size_t channel_count,
        std::uint32_t maximum_microseconds
    ) : channel_count_(channel_count) {
        const double maximum_frames = static_cast<double>(maximum_microseconds)
                                      * static_cast<double>(sample_rate) / 1'000'000.0;
        capacity_frames_ = static_cast<std::size_t>(std::ceil(maximum_frames)) + 4;
        samples_.assign(capacity_frames_ * channel_count_, 0.0F);
    }

    [[nodiscard]] float read(std::size_t channel, double delay_frames) const {
        const double bounded = std::max(1.0, delay_frames);
        const auto whole = static_cast<std::size_t>(std::floor(bounded));
        const float fraction = static_cast<float>(bounded - static_cast<double>(whole));
        const std::size_t recent_frame =
            (cursor_ + capacity_frames_ - whole % capacity_frames_) % capacity_frames_;
        const std::size_t older_frame = (recent_frame + capacity_frames_ - 1) % capacity_frames_;
        const float recent = samples_[recent_frame * channel_count_ + channel];
        const float older = samples_[older_frame * channel_count_ + channel];
        return recent + fraction * (older - recent);
    }

    void write(std::size_t channel, float sample) {
        samples_[cursor_ * channel_count_ + channel] = finite(sample);
    }

    void advance() {
        cursor_ = (cursor_ + 1) % capacity_frames_;
    }

    void reset() {
        std::fill(samples_.begin(), samples_.end(), 0.0F);
        cursor_ = 0;
    }

  private:
    std::size_t channel_count_ = 0;
    std::vector<float> samples_;
    std::size_t capacity_frames_ = 0;
    std::size_t cursor_ = 0;
};

} // namespace

class ChorusVfx::Impl {
  public:
    Impl(ChorusAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        smoothing_(smoothing_coefficient(sample_rate)), delay_(sample_rate, channel_count, 50000) {
        update(adjustment);
        reset();
    }

    void update(ChorusAdjustment adjustment) {
        target_mix_ = static_cast<float>(adjustment.mix_percent) / 100.0F;
        target_rate_ = static_cast<float>(adjustment.rate_millihertz) / 1000.0F;
        target_minimum_delay_ = static_cast<float>(adjustment.minimum_delay_microseconds);
        target_sweep_ = static_cast<float>(adjustment.sweep_microseconds);
        target_stereo_phase_ = static_cast<float>(adjustment.stereo_phase_degrees) / 360.0F;
    }

    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input) {
        const float mix = smooth(mix_, target_mix_, smoothing_);
        const float rate = smooth(rate_, target_rate_, smoothing_);
        const float minimum_delay = smooth(minimum_delay_, target_minimum_delay_, smoothing_);
        const float sweep = smooth(sweep_, target_sweep_, smoothing_);
        const float stereo_phase = smooth(stereo_phase_, target_stereo_phase_, smoothing_);
        std::array<float, 2> output{};
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const double offset = channel == 0 ? 0.0 : static_cast<double>(stereo_phase);
            const double lfo = 0.5 + 0.5 * std::sin(2.0 * std::numbers::pi * (phase_ + offset));
            const double delay_microseconds =
                static_cast<double>(minimum_delay) + static_cast<double>(sweep) * lfo;
            const double delay_frames = delay_microseconds * sample_rate_ / 1'000'000.0;
            const float wet = delay_.read(channel, delay_frames);
            output[channel] = finite(input[channel] + mix * (wet - input[channel]));
            delay_.write(channel, input[channel]);
        }
        if (channel_count_ == 1) {
            output[1] = output[0];
        }
        delay_.advance();
        phase_ += static_cast<double>(rate) / sample_rate_;
        phase_ -= std::floor(phase_);
        return output;
    }

    void reset() {
        delay_.reset();
        phase_ = 0.0;
        mix_ = target_mix_;
        rate_ = target_rate_;
        minimum_delay_ = target_minimum_delay_;
        sweep_ = target_sweep_;
        stereo_phase_ = target_stereo_phase_;
    }

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    float smoothing_ = 1.0F;
    FractionalDelay delay_;
    float mix_ = 0.0F;
    float rate_ = 0.0F;
    float minimum_delay_ = 0.0F;
    float sweep_ = 0.0F;
    float stereo_phase_ = 0.0F;
    float target_mix_ = 0.0F;
    float target_rate_ = 0.0F;
    float target_minimum_delay_ = 0.0F;
    float target_sweep_ = 0.0F;
    float target_stereo_phase_ = 0.0F;
    double phase_ = 0.0;
};

class FlangerVfx::Impl {
  public:
    Impl(FlangerAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        smoothing_(smoothing_coefficient(sample_rate)), delay_(sample_rate, channel_count, 15000) {
        update(adjustment);
        reset();
    }

    void update(FlangerAdjustment adjustment) {
        target_mix_ = static_cast<float>(adjustment.mix_percent) / 100.0F;
        target_rate_ = static_cast<float>(adjustment.rate_millihertz) / 1000.0F;
        target_minimum_delay_ = static_cast<float>(adjustment.minimum_delay_microseconds);
        target_sweep_ = static_cast<float>(adjustment.sweep_microseconds);
        target_feedback_ = static_cast<float>(adjustment.feedback_percent) / 100.0F;
        target_stereo_phase_ = static_cast<float>(adjustment.stereo_phase_degrees) / 360.0F;
    }

    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input) {
        const float mix = smooth(mix_, target_mix_, smoothing_);
        const float rate = smooth(rate_, target_rate_, smoothing_);
        const float minimum_delay = smooth(minimum_delay_, target_minimum_delay_, smoothing_);
        const float sweep = smooth(sweep_, target_sweep_, smoothing_);
        const float feedback = smooth(feedback_, target_feedback_, smoothing_);
        const float stereo_phase = smooth(stereo_phase_, target_stereo_phase_, smoothing_);
        std::array<float, 2> output{};
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const double offset = channel == 0 ? 0.0 : static_cast<double>(stereo_phase);
            const double lfo = 0.5 + 0.5 * std::sin(2.0 * std::numbers::pi * (phase_ + offset));
            const double delay_microseconds =
                static_cast<double>(minimum_delay) + static_cast<double>(sweep) * lfo;
            const double delay_frames = delay_microseconds * sample_rate_ / 1'000'000.0;
            const float wet = finite(delay_.read(channel, delay_frames));
            output[channel] = finite(input[channel] + mix * (wet - input[channel]));
            delay_.write(
                channel,
                std::clamp(
                    finite(input[channel] + feedback * wet),
                    -kMaximumFeedbackSample,
                    kMaximumFeedbackSample
                )
            );
        }
        if (channel_count_ == 1) {
            output[1] = output[0];
        }
        delay_.advance();
        phase_ += static_cast<double>(rate) / sample_rate_;
        phase_ -= std::floor(phase_);
        return output;
    }

    void reset() {
        delay_.reset();
        phase_ = 0.0;
        mix_ = target_mix_;
        rate_ = target_rate_;
        minimum_delay_ = target_minimum_delay_;
        sweep_ = target_sweep_;
        feedback_ = target_feedback_;
        stereo_phase_ = target_stereo_phase_;
    }

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    float smoothing_ = 1.0F;
    FractionalDelay delay_;
    float mix_ = 0.0F;
    float rate_ = 0.0F;
    float minimum_delay_ = 0.0F;
    float sweep_ = 0.0F;
    float feedback_ = 0.0F;
    float stereo_phase_ = 0.0F;
    float target_mix_ = 0.0F;
    float target_rate_ = 0.0F;
    float target_minimum_delay_ = 0.0F;
    float target_sweep_ = 0.0F;
    float target_feedback_ = 0.0F;
    float target_stereo_phase_ = 0.0F;
    double phase_ = 0.0;
};

ChorusVfx::ChorusVfx(
    ChorusAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

ChorusVfx::~ChorusVfx() = default;

void ChorusVfx::update(ChorusAdjustment adjustment) {
    impl_->update(adjustment);
}

std::array<float, 2> ChorusVfx::process(const std::array<float, 2>& input) {
    return impl_->process(input);
}

void ChorusVfx::reset() {
    impl_->reset();
}

FlangerVfx::FlangerVfx(
    FlangerAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

FlangerVfx::~FlangerVfx() = default;

void FlangerVfx::update(FlangerAdjustment adjustment) {
    impl_->update(adjustment);
}

std::array<float, 2> FlangerVfx::process(const std::array<float, 2>& input) {
    return impl_->process(input);
}

void FlangerVfx::reset() {
    impl_->reset();
}

} // namespace echo::audio::detail
