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
constexpr std::size_t kConvolutionPathCount = 4;
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
    PreparedImpulseLayout requested_layout,
    std::span<const float> impulse_ll,
    std::span<const float> impulse_lr,
    std::span<const float> impulse_rl,
    std::span<const float> impulse_rr
) {
    validate_adjustment(adjustment);
    if (sample_rate != kCanonicalSampleRate || channel_count == 0
        || channel_count > kMaximumChannels || impulse_ll.empty()
        || impulse_ll.size() > kMaximumImpulseFrames) {
        throw std::invalid_argument("prepared convolution impulse is unsupported");
    }
    const bool mono = requested_layout == PreparedImpulseLayout::Mono;
    const bool stereo_parallel = requested_layout == PreparedImpulseLayout::StereoParallel;
    const bool true_stereo = requested_layout == PreparedImpulseLayout::TrueStereoLlLrRlRr;
    if ((!mono && !stereo_parallel && !true_stereo)
        || (mono && (!impulse_lr.empty() || !impulse_rl.empty() || !impulse_rr.empty()))
        || (stereo_parallel
            && (channel_count != 2 || !impulse_lr.empty() || !impulse_rl.empty()
                || impulse_rr.size() != impulse_ll.size()))
        || (true_stereo
            && (channel_count != 2 || impulse_lr.size() != impulse_ll.size()
                || impulse_rl.size() != impulse_ll.size()
                || impulse_rr.size() != impulse_ll.size()))) {
        throw std::invalid_argument("prepared convolution impulse is unsupported");
    }
    bool any_nonzero = false;
    for (const std::span<const float> impulse : {impulse_ll, impulse_lr, impulse_rl, impulse_rr}) {
        for (const float sample : impulse) {
            if (!std::isfinite(sample)) {
                throw std::invalid_argument(
                    "prepared convolution impulse contains a non-finite sample"
                );
            }
            any_nonzero = any_nonzero || sample != 0.0F;
        }
    }
    if (!any_nonzero) {
        throw std::invalid_argument("prepared convolution impulse is silent");
    }
    return requested_layout;
}

bool has_nonzero(std::span<const float> impulse) noexcept {
    return std::any_of(impulse.begin(), impulse.end(), [](float sample) { return sample != 0.0F; });
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
        PreparedImpulseLayout requested_layout,
        std::span<const float> impulse_ll,
        std::span<const float> impulse_lr,
        std::span<const float> impulse_rl,
        std::span<const float> impulse_rr
    ) :
        sample_rate_(sample_rate), channel_count_(channel_count), authored_(adjustment),
        layout_(validate_impulse(
            adjustment,
            sample_rate,
            channel_count,
            requested_layout,
            impulse_ll,
            impulse_lr,
            impulse_rl,
            impulse_rr
        )),
        impulse_frames_(impulse_ll.size()),
        input_scratch_{
            std::vector<float>(kScratchFrames, 0.0F),
            std::vector<float>(kScratchFrames, 0.0F)
        },
        output_scratch_{
            std::vector<float>(kScratchFrames, 0.0F),
            std::vector<float>(kScratchFrames, 0.0F),
            std::vector<float>(kScratchFrames, 0.0F),
            std::vector<float>(kScratchFrames, 0.0F)
        } {
        const auto prepare_path = [&](std::size_t path, std::span<const float> impulse) {
            if (!has_nonzero(impulse)) {
                return;
            }
            if (!convolvers_[path].init(kPartitionFrames, impulse.data(), impulse.size())) {
                throw std::runtime_error("cannot prepare convolution bank path");
            }
            prepared_paths_[path] = true;
        };
        prepare_path(0, impulse_ll);
        if (layout_ == PreparedImpulseLayout::TrueStereoLlLrRlRr) {
            prepare_path(1, impulse_lr);
            prepare_path(2, impulse_rl);
        }
        if (channel_count_ == 2) {
            prepare_path(3, layout_ == PreparedImpulseLayout::Mono ? impulse_ll : impulse_rr);
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
            process_path(0, 0, count);
            if (channel_count_ == 2) {
                if (layout_ == PreparedImpulseLayout::TrueStereoLlLrRlRr) {
                    process_path(1, 0, count);
                    process_path(2, 1, count);
                }
                process_path(3, 1, count);
            }
            for (std::size_t frame = 0; frame < count; ++frame) {
                const float wet_mix = enabled_.next() * mix_.next();
                const float wet_gain = centibels_to_gain(wet_gain_centibels_.next());
                const std::size_t base = (processed + frame) * channel_count_;
                const float dry_left = input_scratch_[0][frame];
                const float wet_left =
                    finite(output_scratch_[0][frame] + output_scratch_[2][frame]) * wet_gain;
                samples[base] = dry_left + wet_mix * (wet_left - dry_left);
                if (channel_count_ == 2) {
                    const float dry_right = input_scratch_[1][frame];
                    const float wet_right =
                        finite(output_scratch_[1][frame] + output_scratch_[3][frame]) * wet_gain;
                    samples[base + 1] = dry_right + wet_mix * (wet_right - dry_right);
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
    void
    process_path(std::size_t path, std::size_t input_channel, std::size_t frame_count) noexcept {
        if (prepared_paths_[path]) {
            convolvers_[path].process(
                input_scratch_[input_channel].data(),
                output_scratch_[path].data(),
                frame_count
            );
        } else {
            std::fill_n(output_scratch_[path].data(), frame_count, 0.0F);
        }
    }

    void clear_history() noexcept {
        for (std::size_t path = 0; path < kConvolutionPathCount; ++path) {
            if (prepared_paths_[path]) {
                convolvers_[path].resetState();
            }
            std::fill(output_scratch_[path].begin(), output_scratch_[path].end(), 0.0F);
        }
        for (auto& input : input_scratch_) {
            std::fill(input.begin(), input.end(), 0.0F);
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
    std::array<fftconvolver::FFTConvolver, kConvolutionPathCount> convolvers_;
    std::array<bool, kConvolutionPathCount> prepared_paths_{};
    std::array<std::vector<float>, kMaximumChannels> input_scratch_;
    std::array<std::vector<float>, kConvolutionPathCount> output_scratch_;
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
        std::make_unique<Impl>(
            adjustment,
            sample_rate,
            channel_count,
            impulse_right.empty() ? PreparedImpulseLayout::Mono
                                  : PreparedImpulseLayout::StereoParallel,
            impulse_left,
            std::span<const float>{},
            std::span<const float>{},
            impulse_right
        )
    ) {}

ConvolutionSpaceProcessor::ConvolutionSpaceProcessor(
    ConvolutionSpaceAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count,
    std::span<const float> impulse_ll,
    std::span<const float> impulse_lr,
    std::span<const float> impulse_rl,
    std::span<const float> impulse_rr
) :
    impl_(
        std::make_unique<Impl>(
            adjustment,
            sample_rate,
            channel_count,
            PreparedImpulseLayout::TrueStereoLlLrRlRr,
            impulse_ll,
            impulse_lr,
            impulse_rl,
            impulse_rr
        )
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
