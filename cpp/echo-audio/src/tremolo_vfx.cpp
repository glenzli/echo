#include "tremolo_vfx.hpp"

#include <cmath>
#include <numbers>

namespace echo::audio::detail {
namespace {

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

class TremoloVfx::Impl {
  public:
    Impl(TremoloAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        smoothing_(smoothing_coefficient(sample_rate)) {
        update(adjustment);
        reset();
    }

    void update(TremoloAdjustment adjustment) {
        target_rate_ = static_cast<float>(adjustment.rate_millihertz) / 1000.0F;
        target_depth_ = static_cast<float>(adjustment.depth_percent) / 100.0F;
        target_stereo_phase_ = static_cast<float>(adjustment.stereo_phase_degrees) / 360.0F;
    }

    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input) {
        const float rate = smooth(rate_, target_rate_, smoothing_);
        const float depth = smooth(depth_, target_depth_, smoothing_);
        const float stereo_phase = smooth(stereo_phase_, target_stereo_phase_, smoothing_);
        std::array<float, 2> output{};
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const double offset = channel == 0 ? 0.0 : static_cast<double>(stereo_phase);
            const float lfo = static_cast<float>(
                0.5 + 0.5 * std::sin(2.0 * std::numbers::pi * (phase_ + offset))
            );
            const float gain = 1.0F - depth * lfo;
            output[channel] = finite(input[channel] * gain);
        }
        if (channel_count_ == 1) {
            output[1] = output[0];
        }
        phase_ += static_cast<double>(rate) / sample_rate_;
        phase_ -= std::floor(phase_);
        return output;
    }

    void reset() {
        phase_ = 0.0;
        rate_ = target_rate_;
        depth_ = target_depth_;
        stereo_phase_ = target_stereo_phase_;
    }

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    float smoothing_ = 1.0F;
    float rate_ = 0.0F;
    float depth_ = 0.0F;
    float stereo_phase_ = 0.0F;
    float target_rate_ = 0.0F;
    float target_depth_ = 0.0F;
    float target_stereo_phase_ = 0.0F;
    double phase_ = 0.0;
};

TremoloVfx::TremoloVfx(
    TremoloAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

TremoloVfx::~TremoloVfx() = default;

void TremoloVfx::update(TremoloAdjustment adjustment) {
    impl_->update(adjustment);
}

std::array<float, 2> TremoloVfx::process(const std::array<float, 2>& input) {
    return impl_->process(input);
}

void TremoloVfx::reset() {
    impl_->reset();
}

} // namespace echo::audio::detail
