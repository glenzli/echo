#include "echo/audio/space_processor.hpp"

#include "echo/audio/algorithmic_reverb.hpp"
#include "echo/audio/convolution_space_processor.hpp"
#include "echo/audio/prepared_impulse_response.hpp"

#include <algorithm>
#include <stdexcept>
#include <utility>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::size_t kMaximumProcessingFrames = 4096;

bool same_bank(const SpaceAdjustment& left, const SpaceAdjustment& right) {
    if (left.mode != right.mode) {
        return false;
    }
    return left.mode == SpaceMode::Algorithmic
           || (left.convolution.import_id == right.convolution.import_id
               && left.convolution.source_hash == right.convolution.source_hash
               && left.convolution.prepared_hash == right.convolution.prepared_hash);
}

std::unique_ptr<ConvolutionSpaceProcessor> make_convolution(
    const SpaceAdjustment& adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (adjustment.mode != SpaceMode::Convolution) {
        return nullptr;
    }
    const auto loaded =
        adjustment.convolution.impulse != nullptr
            ? adjustment.convolution.impulse
            : std::make_shared<const LoadedPreparedImpulseResponse>(
                  load_prepared_impulse_response(adjustment.convolution.prepared_path)
              );
    const auto& impulse = *loaded;
    return std::make_unique<ConvolutionSpaceProcessor>(
        adjustment.convolution.adjustment,
        sample_rate,
        channel_count,
        impulse.left,
        impulse.right
    );
}

} // namespace

class SpaceProcessor::Impl {
  public:
    Impl(const SpaceAdjustment& adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), active_(adjustment),
        algorithmic_(adjustment.algorithmic, sample_rate, channel_count),
        convolution_(make_convolution(adjustment, sample_rate, channel_count)),
        dry_(kMaximumProcessingFrames * channel_count, 0.0F),
        transition_frames_(std::max<std::size_t>(1, sample_rate / 50U)) {
        if (sample_rate == 0 || channel_count == 0) {
            throw std::invalid_argument("space processor requires a valid audio layout");
        }
    }

    void update(const SpaceAdjustment& adjustment) {
        if (same_bank(active_, adjustment) && stage_ == Stage::Stable) {
            active_ = adjustment;
            update_active_parameters();
            return;
        }
        pending_ = adjustment;
        pending_convolution_ = make_convolution(adjustment, sample_rate_, channel_count_);
        stage_ = Stage::FadeOut;
        transition_remaining_ = transition_frames_;
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_
            || frame_count > kMaximumProcessingFrames) {
            throw std::invalid_argument("space processor received an invalid audio block");
        }
        std::size_t processed = 0;
        while (processed < frame_count) {
            const std::size_t count =
                stage_ == Stage::Stable ? frame_count - processed
                                        : std::min(frame_count - processed, transition_remaining_);
            float* block = samples + processed * channel_count_;
            std::copy_n(block, count * channel_count_, dry_.data());
            process_active(block, count);
            if (stage_ != Stage::Stable) {
                mix_transition(block, count);
            }
            processed += count;
            if (stage_ != Stage::Stable && transition_remaining_ == 0) {
                advance_transition();
            }
        }
    }

    void reset() {
        algorithmic_.reset();
        if (convolution_ != nullptr) {
            convolution_->reset();
        }
        stage_ = Stage::Stable;
        transition_remaining_ = 0;
    }

  private:
    enum class Stage : std::uint8_t { Stable, FadeOut, FadeIn };

    void process_active(float* samples, std::size_t frame_count) {
        if (active_.mode == SpaceMode::Algorithmic) {
            algorithmic_.process_interleaved(samples, frame_count, channel_count_);
        } else {
            convolution_->process_interleaved(samples, frame_count, channel_count_);
        }
    }

    void mix_transition(float* samples, std::size_t frame_count) {
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const float progress = 1.0F
                                   - static_cast<float>(transition_remaining_)
                                         / static_cast<float>(transition_frames_);
            const float wet = stage_ == Stage::FadeOut ? 1.0F - progress : progress;
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const std::size_t index = frame * channel_count_ + channel;
                samples[index] = dry_[index] + wet * (samples[index] - dry_[index]);
            }
            --transition_remaining_;
        }
    }

    void advance_transition() {
        if (stage_ == Stage::FadeOut) {
            active_ = std::move(pending_);
            convolution_ = std::move(pending_convolution_);
            algorithmic_.update(active_.algorithmic);
            algorithmic_.reset();
            if (convolution_ != nullptr) {
                convolution_->reset();
            }
            stage_ = Stage::FadeIn;
            transition_remaining_ = transition_frames_;
        } else {
            stage_ = Stage::Stable;
        }
    }

    void update_active_parameters() {
        algorithmic_.update(active_.algorithmic);
        if (convolution_ != nullptr) {
            convolution_->update(active_.convolution.adjustment);
        }
    }

    std::uint32_t sample_rate_;
    std::size_t channel_count_;
    SpaceAdjustment active_;
    SpaceAdjustment pending_;
    AlgorithmicReverb algorithmic_;
    std::unique_ptr<ConvolutionSpaceProcessor> convolution_;
    std::unique_ptr<ConvolutionSpaceProcessor> pending_convolution_;
    std::vector<float> dry_;
    std::size_t transition_frames_;
    std::size_t transition_remaining_ = 0;
    Stage stage_ = Stage::Stable;
};

SpaceProcessor::SpaceProcessor(
    const SpaceAdjustment& adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

SpaceProcessor::~SpaceProcessor() = default;

void SpaceProcessor::update(const SpaceAdjustment& adjustment) {
    impl_->update(adjustment);
}

void SpaceProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void SpaceProcessor::reset() {
    impl_->reset();
}

} // namespace echo::audio
