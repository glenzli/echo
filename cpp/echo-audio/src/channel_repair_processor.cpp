#include "echo/audio/channel_repair_processor.hpp"

#include <algorithm>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kTransitionMillis = 20;

} // namespace

ChannelRepairProcessor::ChannelRepairProcessor(
    ChannelRepairAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : sample_rate_(sample_rate), channel_count_(channel_count) {
    if (sample_rate_ == 0 || (channel_count_ != 1 && channel_count_ != 2)) {
        throw std::invalid_argument("channel repair requires mono or stereo audio");
    }
    validate(adjustment);
    transition_length_frames_ =
        std::max<std::size_t>(1, static_cast<std::size_t>(sample_rate_) * kTransitionMillis / 1000);
    target_ = matrix_for(adjustment);
    current_ = target_;
}

void ChannelRepairProcessor::update(ChannelRepairAdjustment adjustment) {
    validate(adjustment);
    target_ = matrix_for(adjustment);
    transition_frames_remaining_ = transition_length_frames_;
    for (std::size_t coefficient = 0; coefficient < current_.size(); ++coefficient) {
        step_[coefficient] = (target_[coefficient] - current_[coefficient])
                             / static_cast<float>(transition_frames_remaining_);
    }
}

void ChannelRepairProcessor::reset() {
    current_ = target_;
    step_.fill(0.0F);
    transition_frames_remaining_ = 0;
}

void ChannelRepairProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
        throw std::invalid_argument("channel repair channel layout changed");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        if (transition_frames_remaining_ > 0) {
            for (std::size_t coefficient = 0; coefficient < current_.size(); ++coefficient) {
                current_[coefficient] += step_[coefficient];
            }
            --transition_frames_remaining_;
            if (transition_frames_remaining_ == 0) {
                current_ = target_;
                step_.fill(0.0F);
            }
        }

        const std::size_t index = frame * channel_count_;
        const float left = samples[index];
        if (channel_count_ == 1) {
            samples[index] = current_[0] * left;
            continue;
        }
        const float right = samples[index + 1];
        samples[index] = current_[0] * left + current_[1] * right;
        samples[index + 1] = current_[2] * left + current_[3] * right;
    }
}

ChannelRepairProcessor::Matrix
ChannelRepairProcessor::matrix_for(ChannelRepairAdjustment adjustment) const {
    if (!adjustment.enabled) {
        return {{1.0F, 0.0F, 0.0F, 1.0F}};
    }
    const float left_polarity = adjustment.invert_left ? -1.0F : 1.0F;
    if (channel_count_ == 1) {
        return {{left_polarity, 0.0F, 0.0F, 1.0F}};
    }
    const float right_polarity = adjustment.invert_right ? -1.0F : 1.0F;
    const float balance = static_cast<float>(adjustment.balance_percent) / 100.0F;
    const float left_gain = balance > 0.0F ? 1.0F - balance : 1.0F;
    const float right_gain = balance < 0.0F ? 1.0F + balance : 1.0F;

    Matrix matrix{};
    if (adjustment.swap_channels) {
        matrix = {{0.0F, left_gain * right_polarity, right_gain * left_polarity, 0.0F}};
    } else {
        matrix = {{left_gain * left_polarity, 0.0F, 0.0F, right_gain * right_polarity}};
    }
    if (adjustment.mono_fold_down) {
        const float source_left = 0.5F * (matrix[0] + matrix[2]);
        const float source_right = 0.5F * (matrix[1] + matrix[3]);
        matrix = {{source_left, source_right, source_left, source_right}};
    }
    return matrix;
}

void ChannelRepairProcessor::validate(ChannelRepairAdjustment adjustment) const {
    if (adjustment.balance_percent < -100 || adjustment.balance_percent > 100) {
        throw std::invalid_argument("channel repair balance must be between -100 and 100 percent");
    }
}

} // namespace echo::audio
