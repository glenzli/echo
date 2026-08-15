#include "echo/audio/stereo_vfx_processor.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;

void validate(StereoVfxParameters parameters, std::uint32_t sample_rate, std::size_t channels) {
    if (parameters.mix_percent > 100 || parameters.width_percent > 200
        || parameters.pan_percent < -100 || parameters.pan_percent > 100
        || sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate || channels == 0
        || channels > 2) {
        throw std::invalid_argument("stereo VFX parameters are outside the supported range");
    }
}

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

float mix(float dry, float wet, float amount) noexcept {
    return finite(dry + amount * (wet - dry));
}

} // namespace

class StereoVfxProcessor::Impl {
  public:
    Impl(StereoVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), parameters_(parameters) {
        validate(parameters, sample_rate, channel_count);
    }

    void update(StereoVfxParameters parameters) {
        validate(parameters, sample_rate_, channel_count_);
        parameters_ = parameters;
    }

    void process(float* samples, std::size_t frames, std::size_t channels) {
        if ((samples == nullptr && frames != 0) || channels != channel_count_) {
            throw std::invalid_argument("stereo VFX channel layout changed");
        }
        if (!parameters_.enabled) {
            return;
        }
        const float wet_mix = static_cast<float>(parameters_.mix_percent) / 100.0F;
        const float width = static_cast<float>(parameters_.width_percent) / 100.0F;
        const float pan = static_cast<float>(parameters_.pan_percent) / 100.0F;
        const float left_gain = std::sqrt(std::clamp(1.0F - pan, 0.0F, 1.0F));
        const float right_gain = std::sqrt(std::clamp(1.0F + pan, 0.0F, 1.0F));
        for (std::size_t frame = 0; frame < frames; ++frame) {
            const std::size_t base = frame * channel_count_;
            const float left = finite(samples[base]);
            if (channel_count_ == 1) {
                samples[base] = left;
                continue;
            }
            const float right = finite(samples[base + 1]);
            const float mid = 0.5F * (left + right);
            const float side = 0.5F * (left - right) * width;
            samples[base] = mix(left, (mid + side) * left_gain, wet_mix);
            samples[base + 1] = mix(right, (mid - side) * right_gain, wet_mix);
        }
    }

    [[nodiscard]] bool bypassed() const noexcept {
        return !parameters_.enabled;
    }
    [[nodiscard]] StereoVfxParameters parameters() const noexcept {
        return parameters_;
    }

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    StereoVfxParameters parameters_{};
};

StereoVfxProcessor::StereoVfxProcessor(
    StereoVfxParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(parameters, sample_rate, channel_count)) {}

StereoVfxProcessor::~StereoVfxProcessor() = default;
void StereoVfxProcessor::update(StereoVfxParameters parameters) {
    impl_->update(parameters);
}
void StereoVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frames,
    std::size_t channels
) {
    impl_->process(samples, frames, channels);
}
void StereoVfxProcessor::reset() noexcept {}
bool StereoVfxProcessor::is_bypassed() const noexcept {
    return impl_->bypassed();
}
StereoVfxParameters StereoVfxProcessor::parameters() const noexcept {
    return impl_->parameters();
}

} // namespace echo::audio
