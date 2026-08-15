#include "echo/audio/auto_wah_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;
constexpr std::size_t kMaximumChannels = 2;
constexpr std::size_t kCoefficientIntervalFrames = 16;

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

void validate(
    AutoWahVfxParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    const std::uint16_t maximum_frequency =
        static_cast<std::uint16_t>(static_cast<float>(sample_rate) * 0.42F);
    if (parameters.mix_percent > 100 || parameters.sensitivity_percent > 100
        || parameters.minimum_frequency_hertz < 80
        || parameters.maximum_frequency_hertz <= parameters.minimum_frequency_hertz
        || parameters.maximum_frequency_hertz > maximum_frequency || parameters.resonance_tenths < 5
        || parameters.resonance_tenths > 50 || sample_rate < kMinimumSampleRate
        || sample_rate > kMaximumSampleRate || channel_count == 0
        || channel_count > kMaximumChannels) {
        throw std::invalid_argument("auto-wah VFX parameters are outside the supported range");
    }
}

float lerp(float from, float to, float amount) noexcept {
    return from + (to - from) * amount;
}

class Biquad {
  public:
    void band_pass(float frequency_hertz, float quality, std::uint32_t sample_rate) noexcept {
        const float omega =
            2.0F * std::numbers::pi_v<float> * frequency_hertz / static_cast<float>(sample_rate);
        const float sine = std::sin(omega);
        const float alpha = sine / (2.0F * quality);
        const float a0 = 1.0F + alpha;
        b0_ = alpha / a0;
        b1_ = 0.0F;
        b2_ = -alpha / a0;
        a1_ = -2.0F * std::cos(omega) / a0;
        a2_ = (1.0F - alpha) / a0;
    }

    [[nodiscard]] float process(float input) noexcept {
        const float output = b0_ * input + z1_;
        z1_ = b1_ * input - a1_ * output + z2_;
        z2_ = b2_ * input - a2_ * output;
        return output;
    }

    void reset() noexcept {
        z1_ = z2_ = 0.0F;
    }

  private:
    float b0_ = 1.0F;
    float b1_ = 0.0F;
    float b2_ = 0.0F;
    float a1_ = 0.0F;
    float a2_ = 0.0F;
    float z1_ = 0.0F;
    float z2_ = 0.0F;
};

} // namespace

class AutoWahVfxProcessor::Impl {
  public:
    Impl(AutoWahVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), parameters_(parameters) {
        validate(parameters, sample_rate, channel_count);
        configure_filters();
    }

    void update(AutoWahVfxParameters parameters) {
        validate(parameters, sample_rate_, channel_count_);
        parameters_ = parameters;
        configure_filters();
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("auto-wah VFX channel layout changed");
        }
        const float mix =
            parameters_.enabled ? static_cast<float>(parameters_.mix_percent) / 100.0F : 0.0F;
        const float sensitivity = static_cast<float>(parameters_.sensitivity_percent) / 100.0F;
        const float attack = std::exp(-1.0F / (0.004F * static_cast<float>(sample_rate_)));
        const float release = std::exp(-1.0F / (0.085F * static_cast<float>(sample_rate_)));
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            float level = 0.0F;
            std::array<float, kMaximumChannels> input{};
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                input[channel] = finite(samples[base + channel]);
                level = std::max(level, std::abs(input[channel]));
            }
            const float coefficient = level > envelope_ ? attack : release;
            envelope_ = coefficient * envelope_ + (1.0F - coefficient) * level;
            if (control_countdown_ == 0) {
                const float normalized =
                    std::clamp(envelope_ * (0.8F + 8.0F * sensitivity), 0.0F, 1.0F);
                current_frequency_ = lerp(
                    static_cast<float>(parameters_.minimum_frequency_hertz),
                    static_cast<float>(parameters_.maximum_frequency_hertz),
                    std::sqrt(normalized)
                );
                configure_filters();
                control_countdown_ = kCoefficientIntervalFrames;
            }
            --control_countdown_;
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float wet = filters_[channel].process(input[channel]) * 2.6F;
                samples[base + channel] = finite(lerp(input[channel], wet, mix));
            }
        }
    }

    void reset() noexcept {
        envelope_ = 0.0F;
        control_countdown_ = 0;
        current_frequency_ = static_cast<float>(parameters_.minimum_frequency_hertz);
        for (auto& filter : filters_) {
            filter.reset();
        }
        configure_filters();
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !parameters_.enabled;
    }
    [[nodiscard]] AutoWahVfxParameters parameters() const noexcept {
        return parameters_;
    }

  private:
    void configure_filters() noexcept {
        const float frequency = std::clamp(
            current_frequency_,
            static_cast<float>(parameters_.minimum_frequency_hertz),
            static_cast<float>(parameters_.maximum_frequency_hertz)
        );
        const float quality = static_cast<float>(parameters_.resonance_tenths) / 10.0F;
        for (auto& filter : filters_) {
            filter.band_pass(frequency, quality, sample_rate_);
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    AutoWahVfxParameters parameters_{};
    std::array<Biquad, kMaximumChannels> filters_{};
    float envelope_ = 0.0F;
    float current_frequency_ = 280.0F;
    std::size_t control_countdown_ = 0;
};

AutoWahVfxProcessor::AutoWahVfxProcessor(
    AutoWahVfxParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(parameters, sample_rate, channel_count)) {}

AutoWahVfxProcessor::~AutoWahVfxProcessor() = default;
void AutoWahVfxProcessor::update(AutoWahVfxParameters parameters) {
    impl_->update(parameters);
}
void AutoWahVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frames,
    std::size_t channels
) {
    impl_->process_interleaved(samples, frames, channels);
}
void AutoWahVfxProcessor::reset() {
    impl_->reset();
}
bool AutoWahVfxProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}
AutoWahVfxParameters AutoWahVfxProcessor::parameters() const noexcept {
    return impl_->parameters();
}

} // namespace echo::audio
