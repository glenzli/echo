#include "echo/audio/rotary_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;
constexpr std::size_t kMaximumChannels = 2;
constexpr float kCrossoverHertz = 800.0F;
constexpr float kSlowHornHertz = 0.8F;
constexpr float kSlowDrumHertz = 0.65F;
constexpr float kFastHornHertz = 6.7F;
constexpr float kFastDrumHertz = 5.8F;
constexpr float kMaximumDelaySeconds = 0.003F;

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

void validate(
    RotaryVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    const bool speed_valid = adjustment.speed == RotaryVfxSpeed::Slow
                             || adjustment.speed == RotaryVfxSpeed::Fast
                             || adjustment.speed == RotaryVfxSpeed::Brake;
    if (!speed_valid || adjustment.mix_percent > 100 || adjustment.motion_percent > 100
        || adjustment.stereo_width_percent > 100 || sample_rate < kMinimumSampleRate
        || sample_rate > kMaximumSampleRate || channel_count == 0
        || channel_count > kMaximumChannels) {
        throw std::invalid_argument("rotary VFX parameters are outside the supported range");
    }
}

RotaryVfxAdjustment
validated(RotaryVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) {
    validate(adjustment, sample_rate, channel_count);
    return adjustment;
}

class LinearRamp {
  public:
    void reset(float value) noexcept {
        current_ = value;
        target_ = value;
        remaining_ = 0;
    }

    void set_target(float value, std::size_t frame_count) noexcept {
        target_ = value;
        remaining_ = current_ == target_ ? 0 : frame_count;
    }

    [[nodiscard]] float next() noexcept {
        if (remaining_ != 0) {
            current_ += (target_ - current_) / static_cast<float>(remaining_);
            --remaining_;
            if (remaining_ == 0) {
                current_ = target_;
            }
        }
        return current_;
    }

  private:
    float current_ = 0.0F;
    float target_ = 0.0F;
    std::size_t remaining_ = 0;
};

struct BiquadCoefficients {
    float b0 = 0.0F;
    float b1 = 0.0F;
    float b2 = 0.0F;
    float a1 = 0.0F;
    float a2 = 0.0F;
};

BiquadCoefficients butterworth_lowpass(std::uint32_t sample_rate) noexcept {
    const float k =
        std::tan(std::numbers::pi_v<float> * kCrossoverHertz / static_cast<float>(sample_rate));
    const float normalization = 1.0F / (1.0F + std::numbers::sqrt2_v<float> * k + k * k);
    return {
        .b0 = k * k * normalization,
        .b1 = 2.0F * k * k * normalization,
        .b2 = k * k * normalization,
        .a1 = 2.0F * (k * k - 1.0F) * normalization,
        .a2 = (1.0F - std::numbers::sqrt2_v<float> * k + k * k) * normalization,
    };
}

BiquadCoefficients butterworth_highpass(std::uint32_t sample_rate) noexcept {
    const float k =
        std::tan(std::numbers::pi_v<float> * kCrossoverHertz / static_cast<float>(sample_rate));
    const float normalization = 1.0F / (1.0F + std::numbers::sqrt2_v<float> * k + k * k);
    return {
        .b0 = normalization,
        .b1 = -2.0F * normalization,
        .b2 = normalization,
        .a1 = 2.0F * (k * k - 1.0F) * normalization,
        .a2 = (1.0F - std::numbers::sqrt2_v<float> * k + k * k) * normalization,
    };
}

class Biquad {
  public:
    explicit Biquad(BiquadCoefficients coefficients) : coefficients_(coefficients) {}

    float process(float input) noexcept {
        const float output = coefficients_.b0 * input + state_1_;
        state_1_ = coefficients_.b1 * input - coefficients_.a1 * output + state_2_;
        state_2_ = coefficients_.b2 * input - coefficients_.a2 * output;
        return output;
    }

    void reset() noexcept {
        state_1_ = 0.0F;
        state_2_ = 0.0F;
    }

  private:
    BiquadCoefficients coefficients_{};
    float state_1_ = 0.0F;
    float state_2_ = 0.0F;
};

class CascadedBiquad {
  public:
    explicit CascadedBiquad(BiquadCoefficients coefficients) :
        first_(coefficients), second_(coefficients) {}

    float process(float input) noexcept {
        return second_.process(first_.process(input));
    }

    void reset() noexcept {
        first_.reset();
        second_.reset();
    }

  private:
    Biquad first_;
    Biquad second_;
};

class FractionalDelay {
  public:
    explicit FractionalDelay(std::size_t capacity) : samples_(capacity, 0.0F) {}

    float process(float input, float delay_frames) noexcept {
        samples_[cursor_] = input;
        const float bounded_delay =
            std::clamp(delay_frames, 1.0F, static_cast<float>(samples_.size() - 2));
        float read_position = static_cast<float>(cursor_) - bounded_delay;
        while (read_position < 0.0F) {
            read_position += static_cast<float>(samples_.size());
        }
        if (read_position >= static_cast<float>(samples_.size())) {
            read_position = 0.0F;
        }
        const std::size_t first = static_cast<std::size_t>(read_position);
        const std::size_t second = (first + 1) % samples_.size();
        const float fraction = read_position - static_cast<float>(first);
        const float output = samples_[first] + fraction * (samples_[second] - samples_[first]);
        cursor_ = (cursor_ + 1) % samples_.size();
        return output;
    }

    void reset() noexcept {
        std::fill(samples_.begin(), samples_.end(), 0.0F);
        cursor_ = 0;
    }

  private:
    std::vector<float> samples_;
    std::size_t cursor_ = 0;
};

float target_hertz(RotaryVfxSpeed speed, bool horn) noexcept {
    switch (speed) {
    case RotaryVfxSpeed::Slow:
        return horn ? kSlowHornHertz : kSlowDrumHertz;
    case RotaryVfxSpeed::Fast:
        return horn ? kFastHornHertz : kFastDrumHertz;
    case RotaryVfxSpeed::Brake:
        return 0.0F;
    }
    return 0.0F;
}

float slew_hertz(
    float current,
    float target,
    float full_scale_hertz,
    float acceleration_seconds,
    float deceleration_seconds,
    std::uint32_t sample_rate
) noexcept {
    const float duration = target > current ? acceleration_seconds : deceleration_seconds;
    const float maximum_step = full_scale_hertz / (duration * static_cast<float>(sample_rate));
    return current + std::clamp(target - current, -maximum_step, maximum_step);
}

float wrap_phase(float phase) noexcept {
    constexpr float full_turn = 2.0F * std::numbers::pi_v<float>;
    if (phase >= full_turn) {
        phase -= full_turn;
    }
    return phase;
}

} // namespace

class RotaryVfxProcessor::Impl {
  public:
    Impl(RotaryVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        authored_(validated(adjustment, sample_rate, channel_count)),
        lowpass_{
            CascadedBiquad(butterworth_lowpass(sample_rate)),
            CascadedBiquad(butterworth_lowpass(sample_rate))
        },
        highpass_{
            CascadedBiquad(butterworth_highpass(sample_rate)),
            CascadedBiquad(butterworth_highpass(sample_rate))
        },
        horn_delay_{
            FractionalDelay(delay_capacity(sample_rate)),
            FractionalDelay(delay_capacity(sample_rate))
        },
        drum_delay_{
            FractionalDelay(delay_capacity(sample_rate)),
            FractionalDelay(delay_capacity(sample_rate))
        } {
        smoothing_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        reset_parameters();
    }

    void update(RotaryVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        enabled_.set_target(adjustment.enabled ? 1.0F : 0.0F, smoothing_frames_);
        mix_.set_target(static_cast<float>(adjustment.mix_percent) / 100.0F, smoothing_frames_);
        motion_.set_target(
            static_cast<float>(adjustment.motion_percent) / 100.0F,
            smoothing_frames_
        );
        width_.set_target(
            static_cast<float>(adjustment.stereo_width_percent) / 100.0F,
            smoothing_frames_
        );
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("rotary VFX channel layout changed");
        }

        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            update_rotor_speeds();
            const float enabled = enabled_.next();
            const float mix = mix_.next();
            const float motion = motion_.next();
            const float width = width_.next();
            const float wet_mix = enabled * mix;
            const std::size_t base = frame * channel_count_;

            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float dry = finite(samples[base + channel]);
                const float input = std::clamp(dry, -16.0F, 16.0F);
                const float low = lowpass_[channel].process(input);
                const float high = highpass_[channel].process(input);
                const float stereo_phase = channel == 0 ? 0.0F : std::numbers::pi_v<float> * width;
                const float horn = rotate_band(
                    high,
                    horn_phase_ + stereo_phase,
                    motion,
                    true,
                    horn_delay_[channel]
                );
                const float drum = rotate_band(
                    low,
                    drum_phase_ + stereo_phase,
                    motion,
                    false,
                    drum_delay_[channel]
                );
                const float rotary = std::clamp(horn + drum, -16.0F, 16.0F);
                const float effect_mix = wet_mix * motion;
                samples[base + channel] =
                    effect_mix == 0.0F
                        ? dry
                        : std::clamp(std::lerp(dry, rotary, effect_mix), -16.0F, 16.0F);
            }

            horn_phase_ = wrap_phase(
                horn_phase_
                + 2.0F * std::numbers::pi_v<float> * horn_hertz_ / static_cast<float>(sample_rate_)
            );
            drum_phase_ = wrap_phase(
                drum_phase_
                + 2.0F * std::numbers::pi_v<float> * drum_hertz_ / static_cast<float>(sample_rate_)
            );
        }
    }

    void reset() noexcept {
        for (auto& filter : lowpass_) {
            filter.reset();
        }
        for (auto& filter : highpass_) {
            filter.reset();
        }
        for (auto& delay : horn_delay_) {
            delay.reset();
        }
        for (auto& delay : drum_delay_) {
            delay.reset();
        }
        horn_phase_ = 0.0F;
        drum_phase_ = 0.0F;
        reset_parameters();
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !authored_.enabled || authored_.mix_percent == 0 || authored_.motion_percent == 0;
    }

    [[nodiscard]] RotaryVfxAdjustment adjustment() const noexcept {
        return authored_;
    }

  private:
    static std::size_t delay_capacity(std::uint32_t sample_rate) noexcept {
        return static_cast<std::size_t>(
                   std::ceil(static_cast<float>(sample_rate) * kMaximumDelaySeconds)
               )
               + 4;
    }

    void reset_parameters() noexcept {
        enabled_.reset(authored_.enabled ? 1.0F : 0.0F);
        mix_.reset(static_cast<float>(authored_.mix_percent) / 100.0F);
        motion_.reset(static_cast<float>(authored_.motion_percent) / 100.0F);
        width_.reset(static_cast<float>(authored_.stereo_width_percent) / 100.0F);
        horn_hertz_ = target_hertz(authored_.speed, true);
        drum_hertz_ = target_hertz(authored_.speed, false);
    }

    void update_rotor_speeds() noexcept {
        const float horn_target = target_hertz(authored_.speed, true);
        const float drum_target = target_hertz(authored_.speed, false);
        horn_hertz_ =
            slew_hertz(horn_hertz_, horn_target, kFastHornHertz, 0.7F, 1.2F, sample_rate_);
        drum_hertz_ =
            slew_hertz(drum_hertz_, drum_target, kFastDrumHertz, 3.5F, 5.0F, sample_rate_);
    }

    float rotate_band(
        float input,
        float phase,
        float motion,
        bool horn,
        FractionalDelay& delay
    ) const noexcept {
        const float phase_cosine = std::cos(phase);
        const float base_seconds = horn ? 0.00125F : 0.0018F;
        const float excursion_seconds = horn ? 0.00055F : 0.00025F;
        const float delay_seconds = base_seconds + motion * excursion_seconds * phase_cosine;
        const float delayed =
            delay.process(input, delay_seconds * static_cast<float>(sample_rate_));
        const float direct_weight = horn ? 0.3F : 0.45F;
        const float amplitude_depth = horn ? 0.42F : 0.24F;
        const float propagated = direct_weight * input + (1.0F - direct_weight) * delayed;
        return propagated * (1.0F + motion * amplitude_depth * phase_cosine);
    }

    std::uint32_t sample_rate_;
    std::size_t channel_count_;
    RotaryVfxAdjustment authored_;
    std::size_t smoothing_frames_ = 1;
    std::array<CascadedBiquad, kMaximumChannels> lowpass_;
    std::array<CascadedBiquad, kMaximumChannels> highpass_;
    std::array<FractionalDelay, kMaximumChannels> horn_delay_;
    std::array<FractionalDelay, kMaximumChannels> drum_delay_;
    LinearRamp enabled_;
    LinearRamp mix_;
    LinearRamp motion_;
    LinearRamp width_;
    float horn_phase_ = 0.0F;
    float drum_phase_ = 0.0F;
    float horn_hertz_ = 0.0F;
    float drum_hertz_ = 0.0F;
};

RotaryVfxProcessor::RotaryVfxProcessor(
    RotaryVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

RotaryVfxProcessor::~RotaryVfxProcessor() = default;

void RotaryVfxProcessor::update(RotaryVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void RotaryVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void RotaryVfxProcessor::reset() {
    impl_->reset();
}

bool RotaryVfxProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}

RotaryVfxAdjustment RotaryVfxProcessor::adjustment() const noexcept {
    return impl_->adjustment();
}

} // namespace echo::audio
