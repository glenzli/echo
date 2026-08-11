#include "phaser_vfx.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>

namespace echo::audio::detail {
namespace {

constexpr std::size_t kStageCount = 6;

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

} // namespace

class PhaserVfx::Impl {
  public:
    Impl(PhaserAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        smoothing_(smoothing_coefficient(sample_rate)) {
        update(adjustment);
        reset();
    }

    void update(PhaserAdjustment adjustment) {
        target_mix_ = static_cast<float>(adjustment.mix_percent) / 100.0F;
        target_rate_ = static_cast<float>(adjustment.rate_millihertz) / 1000.0F;
        target_low_ = static_cast<float>(adjustment.sweep_low_hertz);
        target_high_ = static_cast<float>(adjustment.sweep_high_hertz);
        target_feedback_ = static_cast<float>(adjustment.feedback_percent) / 100.0F;
        target_stereo_phase_ = static_cast<float>(adjustment.stereo_phase_degrees) / 360.0F;
    }

    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input) {
        const float mix = smooth(mix_, target_mix_, smoothing_);
        const float rate = smooth(rate_, target_rate_, smoothing_);
        const float low = smooth(low_, target_low_, smoothing_);
        const float high = smooth(high_, target_high_, smoothing_);
        const float feedback = smooth(feedback_, target_feedback_, smoothing_);
        const float stereo_phase = smooth(stereo_phase_, target_stereo_phase_, smoothing_);
        std::array<float, 2> output{};
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const double offset = channel == 0 ? 0.0 : static_cast<double>(stereo_phase);
            const double lfo = 0.5 + 0.5 * std::sin(2.0 * std::numbers::pi * (phase_ + offset));
            const double safe_low = std::min(
                std::max(20.0, static_cast<double>(low)),
                0.40 * static_cast<double>(sample_rate_)
            );
            const double safe_high = std::max(
                safe_low + 1.0,
                std::min(static_cast<double>(high), 0.45 * static_cast<double>(sample_rate_))
            );
            const double frequency =
                std::exp(std::log(safe_low) + lfo * (std::log(safe_high) - std::log(safe_low)));
            const double tangent =
                std::tan(std::numbers::pi * frequency / static_cast<double>(sample_rate_));
            const float coefficient = static_cast<float>((1.0 - tangent) / (1.0 + tangent));
            float wet = finite(input[channel] + feedback * feedback_state_[channel]);
            for (std::size_t stage = 0; stage < kStageCount; ++stage) {
                const float next = -coefficient * wet + stages_[channel][stage];
                stages_[channel][stage] = finite(wet + coefficient * next);
                wet = finite(next);
            }
            feedback_state_[channel] = wet;
            output[channel] = finite(input[channel] + mix * (wet - input[channel]));
        }
        if (channel_count_ == 1) {
            output[1] = output[0];
        }
        phase_ += static_cast<double>(rate) / sample_rate_;
        phase_ -= std::floor(phase_);
        return output;
    }

    void reset() {
        stages_ = {};
        feedback_state_ = {};
        phase_ = 0.0;
        mix_ = target_mix_;
        rate_ = target_rate_;
        low_ = target_low_;
        high_ = target_high_;
        feedback_ = target_feedback_;
        stereo_phase_ = target_stereo_phase_;
    }

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    float smoothing_ = 1.0F;
    float mix_ = 0.0F;
    float rate_ = 0.0F;
    float low_ = 0.0F;
    float high_ = 0.0F;
    float feedback_ = 0.0F;
    float stereo_phase_ = 0.0F;
    float target_mix_ = 0.0F;
    float target_rate_ = 0.0F;
    float target_low_ = 0.0F;
    float target_high_ = 0.0F;
    float target_feedback_ = 0.0F;
    float target_stereo_phase_ = 0.0F;
    std::array<std::array<float, kStageCount>, 2> stages_{};
    std::array<float, 2> feedback_state_{};
    double phase_ = 0.0;
};

PhaserVfx::PhaserVfx(
    PhaserAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

PhaserVfx::~PhaserVfx() = default;

void PhaserVfx::update(PhaserAdjustment adjustment) {
    impl_->update(adjustment);
}

std::array<float, 2> PhaserVfx::process(const std::array<float, 2>& input) {
    return impl_->process(input);
}

void PhaserVfx::reset() {
    impl_->reset();
}

} // namespace echo::audio::detail
