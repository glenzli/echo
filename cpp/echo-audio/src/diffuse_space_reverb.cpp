#include "echo/audio/diffuse_space_reverb.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::size_t kNetworkSize = 8;
constexpr std::size_t kDiffuserCount = 3;

constexpr std::array<double, kNetworkSize> kHallDelayMilliseconds{
    41.11,
    43.73,
    47.17,
    53.03,
    59.31,
    61.79,
    67.23,
    71.11,
};
constexpr std::array<double, kNetworkSize> kPlateDelayMilliseconds{
    17.89,
    21.31,
    23.93,
    29.11,
    31.67,
    37.09,
    41.23,
    43.81,
};
constexpr std::array<double, kDiffuserCount> kHallDiffuserMilliseconds{4.37, 7.13, 10.79};
constexpr std::array<double, kDiffuserCount> kPlateDiffuserMilliseconds{2.11, 3.71, 5.83};

std::size_t frames_for_millis(double millis, std::uint32_t sample_rate) {
    return std::max<std::size_t>(
        1,
        static_cast<std::size_t>(std::round(millis * static_cast<double>(sample_rate) / 1000.0))
    );
}

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

struct DelayLine {
    std::vector<float> samples;
    std::size_t cursor = 0;
    float damped = 0.0F;

    DelayLine(std::size_t length = 1) : samples(length, 0.0F) {}

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

struct AllpassDiffuser {
    DelayLine delay;
    float gain = 0.0F;

    AllpassDiffuser(std::size_t length = 1, float authored_gain = 0.0F) :
        delay(length), gain(authored_gain) {}

    [[nodiscard]] float process(float input) {
        const float delayed = delay.read();
        const float output = delayed - gain * input;
        delay.write(input + gain * output);
        return finite(output);
    }

    void reset() {
        delay.reset();
    }
};

struct CharacterTuning {
    std::array<double, kNetworkSize> delays{};
    std::array<double, kDiffuserCount> diffusers{};
    float diffusion = 0.0F;
    float injection_gain = 0.0F;
    float output_gain = 0.0F;
    double damping_high_hertz = 0.0;
    double damping_range_hertz = 0.0;
};

CharacterTuning tuning_for(ReverbCharacter character) {
    switch (character) {
    case ReverbCharacter::Hall:
        return CharacterTuning{
            .delays = kHallDelayMilliseconds,
            .diffusers = kHallDiffuserMilliseconds,
            .diffusion = 0.68F,
            .injection_gain = 0.22F,
            .output_gain = 0.21F,
            .damping_high_hertz = 17'000.0,
            .damping_range_hertz = 14'000.0,
        };
    case ReverbCharacter::Plate:
        return CharacterTuning{
            .delays = kPlateDelayMilliseconds,
            .diffusers = kPlateDiffuserMilliseconds,
            .diffusion = 0.76F,
            .injection_gain = 0.19F,
            .output_gain = 0.19F,
            .damping_high_hertz = 19'000.0,
            .damping_range_hertz = 10'000.0,
        };
    case ReverbCharacter::Room:
    case ReverbCharacter::Spring:
        break;
    }
    throw std::invalid_argument("diffuse space reverb only owns hall and plate characters");
}

void validate(ReverbAdjustment adjustment, std::uint32_t sample_rate) {
    if ((adjustment.character != ReverbCharacter::Hall
         && adjustment.character != ReverbCharacter::Plate)
        || sample_rate < 8000 || adjustment.mix_percent > 100 || adjustment.pre_delay_millis > 200
        || adjustment.decay_millis < 100 || adjustment.decay_millis > 12000
        || adjustment.size_percent < 10 || adjustment.size_percent > 100
        || adjustment.damping_percent > 100 || adjustment.low_cut_hertz < 20
        || adjustment.low_cut_hertz > 1000 || adjustment.high_cut_hertz < 1000
        || adjustment.high_cut_hertz > 20000
        || adjustment.low_cut_hertz >= adjustment.high_cut_hertz) {
        throw std::invalid_argument("diffuse space parameters are outside the supported range");
    }
}

} // namespace

class DiffuseSpaceReverb::Impl {
  public:
    Impl(ReverbAdjustment authored, std::uint32_t rate) :
        adjustment_(authored), sample_rate_(rate), tuning_(tuning_for(authored.character)),
        pre_left_(pre_delay_length(authored, rate), 0.0F),
        pre_right_(pre_delay_length(authored, rate), 0.0F) {
        const double size_scale = 0.55 + 0.009 * static_cast<double>(adjustment_.size_percent);
        const double decay_seconds = static_cast<double>(adjustment_.decay_millis) / 1000.0;
        for (std::size_t index = 0; index < kNetworkSize; ++index) {
            const std::size_t length =
                frames_for_millis(tuning_.delays[index] * size_scale, sample_rate_);
            delays_[index] = DelayLine(length);
            const double seconds = static_cast<double>(length) / sample_rate_;
            feedback_[index] = static_cast<float>(std::pow(0.001, seconds / decay_seconds));
        }
        for (std::size_t index = 0; index < kDiffuserCount; ++index) {
            const double right_scale = 1.0 + 0.07 * static_cast<double>(index + 1);
            diffusers_left_[index] = AllpassDiffuser(
                frames_for_millis(tuning_.diffusers[index] * size_scale, sample_rate_),
                tuning_.diffusion
            );
            diffusers_right_[index] = AllpassDiffuser(
                frames_for_millis(
                    tuning_.diffusers[kDiffuserCount - 1 - index] * size_scale * right_scale,
                    sample_rate_
                ),
                tuning_.diffusion
            );
        }

        const double damping_cutoff = tuning_.damping_high_hertz
                                      - tuning_.damping_range_hertz
                                            * static_cast<double>(adjustment_.damping_percent)
                                            / 100.0;
        damping_alpha_ = one_pole_alpha(damping_cutoff, sample_rate_);
        const double wet_high_cut = std::min(
            static_cast<double>(adjustment_.high_cut_hertz),
            0.45 * static_cast<double>(sample_rate_)
        );
        wet_lowpass_alpha_ = one_pole_alpha(wet_high_cut, sample_rate_);
        wet_highpass_decay_ = static_cast<float>(std::exp(
            -2.0 * std::numbers::pi * static_cast<double>(adjustment_.low_cut_hertz) / sample_rate_
        ));
    }

    [[nodiscard]] std::array<float, 2> process_frame(float left, float right) {
        left = finite(left);
        right = finite(right);
        pre_left_[pre_cursor_] = left;
        pre_right_[pre_cursor_] = right;
        const float pre_left = delayed(pre_left_);
        const float pre_right = delayed(pre_right_);
        pre_cursor_ = (pre_cursor_ + 1) % pre_left_.size();

        float diffuse_left = pre_left;
        float diffuse_right = pre_right;
        for (std::size_t index = 0; index < kDiffuserCount; ++index) {
            diffuse_left = diffusers_left_[index].process(diffuse_left);
            diffuse_right = diffusers_right_[index].process(diffuse_right);
        }

        std::array<float, kNetworkSize> tail{};
        float sum = 0.0F;
        for (std::size_t index = 0; index < kNetworkSize; ++index) {
            tail[index] = delays_[index].read();
            sum += tail[index];
        }

        constexpr float kHalfPower = 0.70710678F;
        const float mid = kHalfPower * (diffuse_left + diffuse_right);
        const float side = kHalfPower * (diffuse_left - diffuse_right);
        const std::array<float, kNetworkSize> injection{
            diffuse_left,
            diffuse_right,
            mid,
            side,
            -side,
            -mid,
            kHalfPower * (diffuse_left + side),
            kHalfPower * (diffuse_right - side),
        };
        for (std::size_t index = 0; index < kNetworkSize; ++index) {
            const float householder = tail[index] - 0.25F * sum;
            delays_[index].damped += damping_alpha_ * (householder - delays_[index].damped);
            delays_[index].write(
                tuning_.injection_gain * injection[index] + feedback_[index] * delays_[index].damped
            );
        }

        const float wet_left =
            tuning_.output_gain
            * (tail[0] + tail[1] + tail[2] + tail[3] - tail[4] - tail[5] + tail[6] - tail[7]);
        const float wet_right =
            tuning_.output_gain
            * (tail[0] - tail[1] + tail[2] - tail[3] + tail[4] - tail[5] - tail[6] + tail[7]);
        const float filtered_left = filter_wet(wet_left, 0);
        const float filtered_right = filter_wet(wet_right, 1);
        const float wet =
            adjustment_.enabled ? static_cast<float>(adjustment_.mix_percent) / 100.0F : 0.0F;
        const float dry = 1.0F - wet;
        return {
            finite(dry * left + wet * filtered_left),
            finite(dry * right + wet * filtered_right),
        };
    }

    void reset() {
        std::fill(pre_left_.begin(), pre_left_.end(), 0.0F);
        std::fill(pre_right_.begin(), pre_right_.end(), 0.0F);
        pre_cursor_ = 0;
        for (auto& diffuser : diffusers_left_) {
            diffuser.reset();
        }
        for (auto& diffuser : diffusers_right_) {
            diffuser.reset();
        }
        for (auto& delay : delays_) {
            delay.reset();
        }
        lowpass_state_ = {};
        highpass_state_ = {};
        highpass_input_ = {};
    }

  private:
    static std::size_t pre_delay_length(ReverbAdjustment adjustment, std::uint32_t sample_rate) {
        return static_cast<std::size_t>(std::round(
                   static_cast<double>(adjustment.pre_delay_millis)
                   * static_cast<double>(sample_rate) / 1000.0
               ))
               + 1;
    }

    static float one_pole_alpha(double cutoff_hertz, std::uint32_t sample_rate) {
        return static_cast<float>(
            1.0 - std::exp(-2.0 * std::numbers::pi * cutoff_hertz / sample_rate)
        );
    }

    [[nodiscard]] float delayed(const std::vector<float>& buffer) const {
        return buffer[(pre_cursor_ + 1) % buffer.size()];
    }

    [[nodiscard]] float filter_wet(float sample, std::size_t channel) {
        const float highpassed =
            wet_highpass_decay_ * (highpass_state_[channel] + sample - highpass_input_[channel]);
        highpass_input_[channel] = sample;
        highpass_state_[channel] = highpassed;
        lowpass_state_[channel] += wet_lowpass_alpha_ * (highpassed - lowpass_state_[channel]);
        return lowpass_state_[channel];
    }

    ReverbAdjustment adjustment_;
    std::uint32_t sample_rate_;
    CharacterTuning tuning_;
    std::vector<float> pre_left_;
    std::vector<float> pre_right_;
    std::size_t pre_cursor_ = 0;
    std::array<AllpassDiffuser, kDiffuserCount> diffusers_left_{};
    std::array<AllpassDiffuser, kDiffuserCount> diffusers_right_{};
    std::array<DelayLine, kNetworkSize> delays_{};
    std::array<float, kNetworkSize> feedback_{};
    float damping_alpha_ = 1.0F;
    float wet_lowpass_alpha_ = 1.0F;
    float wet_highpass_decay_ = 0.0F;
    std::array<float, 2> lowpass_state_{};
    std::array<float, 2> highpass_state_{};
    std::array<float, 2> highpass_input_{};
};

DiffuseSpaceReverb::DiffuseSpaceReverb(ReverbAdjustment adjustment, std::uint32_t sample_rate) {
    validate(adjustment, sample_rate);
    impl_ = std::make_unique<Impl>(adjustment, sample_rate);
}

DiffuseSpaceReverb::~DiffuseSpaceReverb() = default;

std::array<float, 2> DiffuseSpaceReverb::process_frame(float left, float right) {
    return impl_->process_frame(left, right);
}

void DiffuseSpaceReverb::reset() {
    impl_->reset();
}

} // namespace echo::audio
