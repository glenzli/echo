#include "echo/audio/drive_vfx_processor.hpp"

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
constexpr std::size_t kOversamplingFactor = 2;
constexpr std::size_t kFirTapCount = 65;
constexpr std::uint16_t kMaximumDriveCentibels = 3600;
constexpr std::uint16_t kMinimumToneHertz = 500;
constexpr std::uint16_t kMaximumToneHertz = 16000;
constexpr std::int16_t kMinimumOutputGainCentibels = -2400;
constexpr std::int16_t kMaximumOutputGainCentibels = 600;
constexpr double kKaiserBeta = 8.6;

static_assert((kFirTapCount - 1) / kOversamplingFactor == DriveVfxProcessor::latency_frames());

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

void validate(DriveVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) {
    const bool character_valid = adjustment.character == DriveVfxCharacter::SoftClip
                                 || adjustment.character == DriveVfxCharacter::Overdrive
                                 || adjustment.character == DriveVfxCharacter::Fuzz;
    if (!character_valid || adjustment.mix_percent > 100
        || adjustment.drive_centibels > kMaximumDriveCentibels
        || adjustment.tone_hertz < kMinimumToneHertz || adjustment.tone_hertz > kMaximumToneHertz
        || adjustment.output_gain_centibels < kMinimumOutputGainCentibels
        || adjustment.output_gain_centibels > kMaximumOutputGainCentibels
        || sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate
        || channel_count == 0 || channel_count > kMaximumChannels) {
        throw std::invalid_argument("drive VFX parameters are outside the supported range");
    }
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
        if (remaining_ == 0) {
            return current_;
        }
        current_ += (target_ - current_) / static_cast<float>(remaining_);
        --remaining_;
        if (remaining_ == 0) {
            current_ = target_;
        }
        return current_;
    }

  private:
    float current_ = 0.0F;
    float target_ = 0.0F;
    std::size_t remaining_ = 0;
};

double bessel_i0(double value) noexcept {
    const double half = value * 0.5;
    double term = 1.0;
    double sum = 1.0;
    for (std::size_t order = 1; order <= 24; ++order) {
        const double divisor = static_cast<double>(order);
        term *= (half * half) / (divisor * divisor);
        sum += term;
        if (term < sum * 1.0E-16) {
            break;
        }
    }
    return sum;
}

std::array<float, kFirTapCount> design_lowpass(std::uint32_t sample_rate) {
    std::array<float, kFirTapCount> coefficients{};
    const double base_rate = static_cast<double>(sample_rate);
    const double high_rate = base_rate * static_cast<double>(kOversamplingFactor);
    const double cutoff_hertz = std::min(static_cast<double>(kMaximumToneHertz), base_rate * 0.45);
    const double normalized_cutoff = cutoff_hertz / high_rate;
    const double radius = static_cast<double>((kFirTapCount - 1) / 2);
    const double window_denominator = bessel_i0(kKaiserBeta);
    double sum = 0.0;

    for (std::size_t tap = 0; tap < kFirTapCount; ++tap) {
        const double offset = static_cast<double>(tap) - radius;
        const double ideal = offset == 0.0
                                 ? 2.0 * normalized_cutoff
                                 : std::sin(2.0 * std::numbers::pi * normalized_cutoff * offset)
                                       / (std::numbers::pi * offset);
        const double ratio = offset / radius;
        const double window = bessel_i0(kKaiserBeta * std::sqrt(std::max(0.0, 1.0 - ratio * ratio)))
                              / window_denominator;
        coefficients[tap] = static_cast<float>(ideal * window);
        sum += static_cast<double>(coefficients[tap]);
    }
    for (float& coefficient : coefficients) {
        coefficient = static_cast<float>(static_cast<double>(coefficient) / sum);
    }
    return coefficients;
}

std::array<float, kFirTapCount> validated_lowpass(
    DriveVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    validate(adjustment, sample_rate, channel_count);
    return design_lowpass(sample_rate);
}

class StereoFir {
  public:
    explicit StereoFir(std::array<float, kFirTapCount> coefficients) :
        coefficients_(coefficients) {}

    [[nodiscard]] std::array<float, kMaximumChannels>
    process(const std::array<float, kMaximumChannels>& input, std::size_t channel_count) noexcept {
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            history_[channel][cursor_] = input[channel];
        }

        std::array<float, kMaximumChannels> output{};
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            double sum = 0.0;
            std::size_t index = cursor_;
            for (const float coefficient : coefficients_) {
                sum += static_cast<double>(coefficient)
                       * static_cast<double>(history_[channel][index]);
                index = index == 0 ? kFirTapCount - 1 : index - 1;
            }
            output[channel] = static_cast<float>(sum);
        }

        cursor_ = (cursor_ + 1) % kFirTapCount;
        return output;
    }

    void reset() noexcept {
        for (auto& channel : history_) {
            channel.fill(0.0F);
        }
        cursor_ = 0;
    }

  private:
    std::array<float, kFirTapCount> coefficients_{};
    std::array<std::array<float, kFirTapCount>, kMaximumChannels> history_{};
    std::size_t cursor_ = 0;
};

float centibels_to_gain(float centibels) noexcept {
    return std::pow(10.0F, centibels / 2000.0F);
}

float soft_clip(float sample, float drive) noexcept {
    const float denominator = std::tanh(drive);
    return std::tanh(sample * drive) / denominator;
}

float overdrive(float sample, float drive) noexcept {
    const float denominator = std::atan(drive);
    return std::atan(sample * drive) / denominator;
}

float fuzz(float sample, float drive) noexcept {
    return std::clamp(sample * drive * 1.5F, -1.0F, 1.0F);
}

float character_weight(DriveVfxCharacter selected, DriveVfxCharacter character) noexcept {
    return selected == character ? 1.0F : 0.0F;
}

} // namespace

class DriveVfxProcessor::Impl {
  public:
    Impl(DriveVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), authored_(adjustment),
        upsampler_(validated_lowpass(adjustment, sample_rate, channel_count)),
        downsampler_(design_lowpass(sample_rate)) {
        smoothing_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        reset_parameters();
    }

    void update(DriveVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        enabled_.set_target(adjustment.enabled ? 1.0F : 0.0F, smoothing_frames_);
        mix_.set_target(static_cast<float>(adjustment.mix_percent) / 100.0F, smoothing_frames_);
        drive_centibels_.set_target(
            static_cast<float>(adjustment.drive_centibels),
            smoothing_frames_
        );
        tone_hertz_.set_target(static_cast<float>(adjustment.tone_hertz), smoothing_frames_);
        output_gain_centibels_.set_target(
            static_cast<float>(adjustment.output_gain_centibels),
            smoothing_frames_
        );
        soft_clip_weight_.set_target(
            character_weight(adjustment.character, DriveVfxCharacter::SoftClip),
            smoothing_frames_
        );
        overdrive_weight_.set_target(
            character_weight(adjustment.character, DriveVfxCharacter::Overdrive),
            smoothing_frames_
        );
        fuzz_weight_.set_target(
            character_weight(adjustment.character, DriveVfxCharacter::Fuzz),
            smoothing_frames_
        );
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("drive VFX channel layout changed");
        }

        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            const std::array<float, kMaximumChannels> input{
                finite(samples[base]),
                channel_count_ == 1 ? finite(samples[base]) : finite(samples[base + 1]),
            };
            const auto dry = delay_dry(input);
            std::array<float, kMaximumChannels> bounded_input{};
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                bounded_input[channel] = std::clamp(input[channel], -8.0F, 8.0F);
            }

            const float drive = centibels_to_gain(drive_centibels_.next());
            const float soft_weight = soft_clip_weight_.next();
            const float overdrive_weight = overdrive_weight_.next();
            const float fuzz_character_weight = fuzz_weight_.next();
            const float output_gain = centibels_to_gain(output_gain_centibels_.next());
            const float tone_hertz =
                std::min(tone_hertz_.next(), static_cast<float>(sample_rate_) * 0.45F);

            std::array<float, kMaximumChannels> wet{};
            for (std::size_t phase = 0; phase < kOversamplingFactor; ++phase) {
                std::array<float, kMaximumChannels> high_input{};
                if (phase == 0) {
                    for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                        high_input[channel] = bounded_input[channel];
                    }
                }
                auto high = upsampler_.process(high_input, channel_count_);
                for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                    const float sample = high[channel] * static_cast<float>(kOversamplingFactor);
                    high[channel] = soft_weight * soft_clip(sample, drive)
                                    + overdrive_weight * overdrive(sample, drive)
                                    + fuzz_character_weight * fuzz(sample, drive);
                }
                const auto filtered = downsampler_.process(high, channel_count_);
                if (phase == 0) {
                    wet = filtered;
                }
            }

            const float pole = std::exp(
                -2.0F * std::numbers::pi_v<float> * tone_hertz / static_cast<float>(sample_rate_)
            );
            const float wet_mix = enabled_.next() * mix_.next();
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                tone_state_[channel] = (1.0F - pole) * wet[channel] + pole * tone_state_[channel];
                const float shaped = tone_state_[channel] * output_gain;
                samples[base + channel] =
                    finite((1.0F - wet_mix) * dry[channel] + wet_mix * shaped);
            }
        }
    }

    void reset() noexcept {
        upsampler_.reset();
        downsampler_.reset();
        for (auto& frame : dry_delay_) {
            frame.fill(0.0F);
        }
        dry_cursor_ = 0;
        tone_state_.fill(0.0F);
        reset_parameters();
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !authored_.enabled;
    }

    [[nodiscard]] DriveVfxAdjustment adjustment() const noexcept {
        return authored_;
    }

  private:
    [[nodiscard]] std::array<float, kMaximumChannels>
    delay_dry(const std::array<float, kMaximumChannels>& input) noexcept {
        const auto output = dry_delay_[dry_cursor_];
        dry_delay_[dry_cursor_] = input;
        dry_cursor_ = (dry_cursor_ + 1) % dry_delay_.size();
        return output;
    }

    void reset_parameters() noexcept {
        enabled_.reset(authored_.enabled ? 1.0F : 0.0F);
        mix_.reset(static_cast<float>(authored_.mix_percent) / 100.0F);
        drive_centibels_.reset(static_cast<float>(authored_.drive_centibels));
        tone_hertz_.reset(static_cast<float>(authored_.tone_hertz));
        output_gain_centibels_.reset(static_cast<float>(authored_.output_gain_centibels));
        soft_clip_weight_.reset(character_weight(authored_.character, DriveVfxCharacter::SoftClip));
        overdrive_weight_.reset(
            character_weight(authored_.character, DriveVfxCharacter::Overdrive)
        );
        fuzz_weight_.reset(character_weight(authored_.character, DriveVfxCharacter::Fuzz));
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t smoothing_frames_ = 1;
    DriveVfxAdjustment authored_;
    StereoFir upsampler_;
    StereoFir downsampler_;
    std::array<std::array<float, kMaximumChannels>, DriveVfxProcessor::latency_frames()>
        dry_delay_{};
    std::size_t dry_cursor_ = 0;
    std::array<float, kMaximumChannels> tone_state_{};
    LinearRamp enabled_;
    LinearRamp mix_;
    LinearRamp drive_centibels_;
    LinearRamp tone_hertz_;
    LinearRamp output_gain_centibels_;
    LinearRamp soft_clip_weight_;
    LinearRamp overdrive_weight_;
    LinearRamp fuzz_weight_;
};

DriveVfxProcessor::DriveVfxProcessor(
    DriveVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

DriveVfxProcessor::~DriveVfxProcessor() = default;

void DriveVfxProcessor::update(DriveVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void DriveVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void DriveVfxProcessor::reset() {
    impl_->reset();
}

bool DriveVfxProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}

DriveVfxAdjustment DriveVfxProcessor::adjustment() const noexcept {
    return impl_->adjustment();
}

} // namespace echo::audio
