#include "echo/audio/digital_degrade_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;
constexpr std::uint8_t kMinimumBitDepth = 2;
constexpr std::uint8_t kMaximumBitDepth = 16;
constexpr std::uint16_t kMinimumTargetRateHertz = 1000;
constexpr std::uint16_t kMaximumTargetRateHertz = 24000;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

void validate(
    DigitalDegradeVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    const bool character_valid =
        adjustment.character == DigitalDegradeVfxCharacter::Bitcrusher
        || adjustment.character == DigitalDegradeVfxCharacter::SampleRateReduction
        || adjustment.character == DigitalDegradeVfxCharacter::LoFi;
    const bool bitcrusher_valid = adjustment.bitcrusher.bit_depth >= kMinimumBitDepth
                                  && adjustment.bitcrusher.bit_depth <= kMaximumBitDepth;
    const bool sample_rate_reduction_valid =
        adjustment.sample_rate_reduction.target_rate_hertz >= kMinimumTargetRateHertz
        && adjustment.sample_rate_reduction.target_rate_hertz <= kMaximumTargetRateHertz;
    if (!character_valid || !bitcrusher_valid || !sample_rate_reduction_valid
        || adjustment.mix_percent > 100 || sample_rate < kMinimumSampleRate
        || sample_rate > kMaximumSampleRate || channel_count == 0 || channel_count > 2) {
        throw std::invalid_argument(
            "digital degrade VFX parameters are outside the supported range"
        );
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

float character_weight(DigitalDegradeVfxCharacter selected, DigitalDegradeVfxCharacter character) {
    return selected == character ? 1.0F : 0.0F;
}

float quantize(float sample, float bit_depth) {
    const float clamped = std::clamp(finite(sample), -1.0F, 1.0F);
    const float positive_levels = std::exp2(bit_depth - 1.0F) - 1.0F;
    return std::round(clamped * positive_levels) / positive_levels;
}

} // namespace

class DigitalDegradeVfxProcessor::Impl {
  public:
    Impl(
        DigitalDegradeVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    ) : sample_rate_(sample_rate), channel_count_(channel_count) {
        validate(adjustment, sample_rate_, channel_count_);
        smoothing_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        authored_ = adjustment;
        reset_parameters();
    }

    void update(DigitalDegradeVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        enabled_.set_target(adjustment.enabled ? 1.0F : 0.0F, smoothing_frames_);
        mix_.set_target(static_cast<float>(adjustment.mix_percent) / 100.0F, smoothing_frames_);
        bit_depth_.set_target(
            static_cast<float>(adjustment.bitcrusher.bit_depth),
            smoothing_frames_
        );
        target_rate_hertz_.set_target(
            static_cast<float>(adjustment.sample_rate_reduction.target_rate_hertz),
            smoothing_frames_
        );
        bitcrusher_weight_.set_target(
            character_weight(adjustment.character, DigitalDegradeVfxCharacter::Bitcrusher),
            smoothing_frames_
        );
        sample_rate_reduction_weight_.set_target(
            character_weight(adjustment.character, DigitalDegradeVfxCharacter::SampleRateReduction),
            smoothing_frames_
        );
        lofi_weight_.set_target(
            character_weight(adjustment.character, DigitalDegradeVfxCharacter::LoFi),
            smoothing_frames_
        );
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("digital degrade VFX channel layout changed");
        }

        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            const std::array<float, 2> input{
                finite(samples[base]),
                channel_count_ == 1 ? finite(samples[base]) : finite(samples[base + 1]),
            };

            const float bit_depth = bit_depth_.next();
            const float target_rate =
                std::min(target_rate_hertz_.next(), static_cast<float>(sample_rate_));
            capture_if_due(input);

            const float bitcrusher_weight = bitcrusher_weight_.next();
            const float sample_rate_reduction_weight = sample_rate_reduction_weight_.next();
            const float lofi_weight = lofi_weight_.next();
            const float weight_sum = bitcrusher_weight + sample_rate_reduction_weight + lofi_weight;
            const float wet_mix = enabled_.next() * mix_.next();

            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float bitcrushed = quantize(input[channel], bit_depth);
                const float held = held_[channel];
                const float lofi = quantize(held, bit_depth);
                const float wet = (bitcrusher_weight * bitcrushed
                                   + sample_rate_reduction_weight * held + lofi_weight * lofi)
                                  / weight_sum;
                samples[base + channel] = finite(input[channel] + wet_mix * (wet - input[channel]));
            }

            hold_phase_ += static_cast<double>(target_rate) / static_cast<double>(sample_rate_);
        }
    }

    void reset() {
        held_.fill(0.0F);
        hold_phase_ = 0.0;
        has_held_sample_ = false;
        reset_parameters();
    }

    [[nodiscard]] bool is_bypassed() const {
        return !authored_.enabled;
    }

  private:
    void capture_if_due(const std::array<float, 2>& input) {
        if (!has_held_sample_ || hold_phase_ >= 1.0) {
            held_ = input;
            has_held_sample_ = true;
            hold_phase_ -= std::floor(hold_phase_);
        }
    }

    void reset_parameters() {
        enabled_.reset(authored_.enabled ? 1.0F : 0.0F);
        mix_.reset(static_cast<float>(authored_.mix_percent) / 100.0F);
        bit_depth_.reset(static_cast<float>(authored_.bitcrusher.bit_depth));
        target_rate_hertz_.reset(
            static_cast<float>(authored_.sample_rate_reduction.target_rate_hertz)
        );
        bitcrusher_weight_.reset(
            character_weight(authored_.character, DigitalDegradeVfxCharacter::Bitcrusher)
        );
        sample_rate_reduction_weight_.reset(
            character_weight(authored_.character, DigitalDegradeVfxCharacter::SampleRateReduction)
        );
        lofi_weight_.reset(character_weight(authored_.character, DigitalDegradeVfxCharacter::LoFi));
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t smoothing_frames_ = 1;
    DigitalDegradeVfxAdjustment authored_;
    std::array<float, 2> held_{};
    double hold_phase_ = 0.0;
    bool has_held_sample_ = false;
    LinearRamp enabled_;
    LinearRamp mix_;
    LinearRamp bit_depth_;
    LinearRamp target_rate_hertz_;
    LinearRamp bitcrusher_weight_;
    LinearRamp sample_rate_reduction_weight_;
    LinearRamp lofi_weight_;
};

DigitalDegradeVfxProcessor::DigitalDegradeVfxProcessor(
    DigitalDegradeVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

DigitalDegradeVfxProcessor::~DigitalDegradeVfxProcessor() = default;

void DigitalDegradeVfxProcessor::update(DigitalDegradeVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void DigitalDegradeVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void DigitalDegradeVfxProcessor::reset() {
    impl_->reset();
}

bool DigitalDegradeVfxProcessor::is_bypassed() const {
    return impl_->is_bypassed();
}

} // namespace echo::audio
