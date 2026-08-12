#include "echo/audio/convolution_space_processor.hpp"

#include "fftconvolver/FFTConvolver.h"

#include <algorithm>
#include <array>
#include <cmath>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kCanonicalSampleRate = 48000;
constexpr std::size_t kMaximumChannels = 2;
constexpr std::size_t kPartitionFrames = 256;
constexpr std::size_t kScratchFrames = 4096;
constexpr std::size_t kMaximumImpulseFrames = kCanonicalSampleRate * 5U;
constexpr std::int16_t kMinimumWetGainCentibels = -2400;
constexpr std::int16_t kMaximumWetGainCentibels = 1200;

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

void validate_adjustment(ConvolutionSpaceAdjustment adjustment) {
    if (adjustment.mix_percent > 100 || adjustment.wet_gain_centibels < kMinimumWetGainCentibels
        || adjustment.wet_gain_centibels > kMaximumWetGainCentibels) {
        throw std::invalid_argument("convolution space parameters are outside the supported range");
    }
}

PreparedImpulseLayout validate_impulse(
    ConvolutionSpaceAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count,
    std::span<const float> impulse_left,
    std::span<const float> impulse_right
) {
    validate_adjustment(adjustment);
    if (sample_rate != kCanonicalSampleRate || channel_count == 0
        || channel_count > kMaximumChannels || impulse_left.empty()
        || impulse_left.size() > kMaximumImpulseFrames
        || (!impulse_right.empty() && channel_count != 2)
        || (!impulse_right.empty() && impulse_right.size() != impulse_left.size())) {
        throw std::invalid_argument("prepared convolution impulse is unsupported");
    }
    bool any_nonzero = false;
    for (const float sample : impulse_left) {
        if (!std::isfinite(sample)) {
            throw std::invalid_argument(
                "prepared convolution impulse contains a non-finite sample"
            );
        }
        any_nonzero = any_nonzero || sample != 0.0F;
    }
    for (const float sample : impulse_right) {
        if (!std::isfinite(sample)) {
            throw std::invalid_argument(
                "prepared convolution impulse contains a non-finite sample"
            );
        }
        any_nonzero = any_nonzero || sample != 0.0F;
    }
    if (!any_nonzero) {
        throw std::invalid_argument("prepared convolution impulse is silent");
    }
    return impulse_right.empty() ? PreparedImpulseLayout::Mono
                                 : PreparedImpulseLayout::StereoParallel;
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

    [[nodiscard]] bool settled_at(float value) const noexcept {
        return remaining_ == 0 && current_ == value && target_ == value;
    }

  private:
    float current_ = 0.0F;
    float target_ = 0.0F;
    std::size_t remaining_ = 0;
};

float centibels_to_gain(float centibels) noexcept {
    return std::pow(10.0F, centibels / 2000.0F);
}

} // namespace

class ConvolutionSpaceProcessor::Impl {
  public:
    Impl(
        ConvolutionSpaceAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count,
        std::span<const float> impulse_left,
        std::span<const float> impulse_right
    ) :
        sample_rate_(sample_rate), channel_count_(channel_count), authored_(adjustment),
        layout_(
            validate_impulse(adjustment, sample_rate, channel_count, impulse_left, impulse_right)
        ),
        impulse_frames_(impulse_left.size()),
        input_scratch_{
            std::vector<float>(kScratchFrames, 0.0F),
            std::vector<float>(kScratchFrames, 0.0F)
        },
        output_scratch_{
            std::vector<float>(kScratchFrames, 0.0F),
            std::vector<float>(kScratchFrames, 0.0F)
        } {
        if (!convolvers_[0].init(kPartitionFrames, impulse_left.data(), impulse_left.size())) {
            throw std::runtime_error("cannot prepare left convolution bank");
        }
        if (channel_count_ == 2) {
            const auto right = impulse_right.empty() ? impulse_left : impulse_right;
            if (!convolvers_[1].init(kPartitionFrames, right.data(), right.size())) {
                throw std::runtime_error("cannot prepare right convolution bank");
            }
        }
        smoothing_frames_ = sample_rate_ / 50U;
        reset_parameters();
    }

    void update(ConvolutionSpaceAdjustment adjustment) {
        validate_adjustment(adjustment);
        authored_ = adjustment;
        enabled_.set_target(adjustment.enabled ? 1.0F : 0.0F, smoothing_frames_);
        mix_.set_target(static_cast<float>(adjustment.mix_percent) / 100.0F, smoothing_frames_);
        wet_gain_centibels_.set_target(
            static_cast<float>(adjustment.wet_gain_centibels),
            smoothing_frames_
        );
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("convolution space channel layout changed");
        }
        if (enabled_.settled_at(0.0F) || mix_.settled_at(0.0F)) {
            if (!history_cleared_) {
                clear_history();
            }
            for (std::size_t sample = 0; sample < frame_count * channel_count_; ++sample) {
                samples[sample] = finite(samples[sample]);
            }
            return;
        }
        history_cleared_ = false;
        std::size_t processed = 0;
        while (processed < frame_count) {
            const std::size_t count = std::min(kScratchFrames, frame_count - processed);
            for (std::size_t frame = 0; frame < count; ++frame) {
                const std::size_t base = (processed + frame) * channel_count_;
                for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                    input_scratch_[channel][frame] = finite(samples[base + channel]);
                }
            }
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                convolvers_[channel].process(
                    input_scratch_[channel].data(),
                    output_scratch_[channel].data(),
                    count
                );
            }
            for (std::size_t frame = 0; frame < count; ++frame) {
                const float wet_mix = enabled_.next() * mix_.next();
                const float wet_gain = centibels_to_gain(wet_gain_centibels_.next());
                const std::size_t base = (processed + frame) * channel_count_;
                for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                    const float dry = input_scratch_[channel][frame];
                    const float wet = finite(output_scratch_[channel][frame]) * wet_gain;
                    samples[base + channel] = dry + wet_mix * (wet - dry);
                }
            }
            processed += count;
        }
    }

    void reset() noexcept {
        clear_history();
        reset_parameters();
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return (enabled_.settled_at(0.0F) || mix_.settled_at(0.0F)) && history_cleared_;
    }

    [[nodiscard]] ConvolutionSpaceAdjustment adjustment() const noexcept {
        return authored_;
    }

    [[nodiscard]] PreparedImpulseLayout impulse_layout() const noexcept {
        return layout_;
    }

    [[nodiscard]] std::size_t impulse_frames() const noexcept {
        return impulse_frames_;
    }

  private:
    void clear_history() noexcept {
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            convolvers_[channel].resetState();
            std::fill(input_scratch_[channel].begin(), input_scratch_[channel].end(), 0.0F);
            std::fill(output_scratch_[channel].begin(), output_scratch_[channel].end(), 0.0F);
        }
        history_cleared_ = true;
    }
    void reset_parameters() noexcept {
        enabled_.reset(authored_.enabled ? 1.0F : 0.0F);
        mix_.reset(static_cast<float>(authored_.mix_percent) / 100.0F);
        wet_gain_centibels_.reset(static_cast<float>(authored_.wet_gain_centibels));
        history_cleared_ = !authored_.enabled || authored_.mix_percent == 0;
    }

    std::uint32_t sample_rate_;
    std::size_t channel_count_;
    ConvolutionSpaceAdjustment authored_;
    PreparedImpulseLayout layout_;
    std::size_t impulse_frames_;
    std::size_t smoothing_frames_ = 1;
    std::array<fftconvolver::FFTConvolver, kMaximumChannels> convolvers_;
    std::array<std::vector<float>, kMaximumChannels> input_scratch_;
    std::array<std::vector<float>, kMaximumChannels> output_scratch_;
    LinearRamp enabled_;
    LinearRamp mix_;
    LinearRamp wet_gain_centibels_;
    bool history_cleared_ = false;
};

ConvolutionSpaceProcessor::ConvolutionSpaceProcessor(
    ConvolutionSpaceAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count,
    std::span<const float> impulse_left,
    std::span<const float> impulse_right
) :
    impl_(
        std::make_unique<Impl>(adjustment, sample_rate, channel_count, impulse_left, impulse_right)
    ) {}

ConvolutionSpaceProcessor::~ConvolutionSpaceProcessor() = default;

void ConvolutionSpaceProcessor::update(ConvolutionSpaceAdjustment adjustment) {
    impl_->update(adjustment);
}

void ConvolutionSpaceProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void ConvolutionSpaceProcessor::reset() {
    impl_->reset();
}

bool ConvolutionSpaceProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}

ConvolutionSpaceAdjustment ConvolutionSpaceProcessor::adjustment() const noexcept {
    return impl_->adjustment();
}

PreparedImpulseLayout ConvolutionSpaceProcessor::impulse_layout() const noexcept {
    return impl_->impulse_layout();
}

std::size_t ConvolutionSpaceProcessor::impulse_frames() const noexcept {
    return impl_->impulse_frames();
}

std::size_t ConvolutionSpaceProcessor::tail_frames() const noexcept {
    return impulse_frames() - 1;
}

} // namespace echo::audio
