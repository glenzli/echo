#include "echo/audio/tape_vfx_processor.hpp"

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
constexpr float kMaximumDelayMillis = 18.0F;

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

void validate(TapeVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) {
    if (parameters.mix_percent > 100 || parameters.saturation_percent > 100
        || parameters.wow_flutter_percent > 100 || parameters.dropout_percent > 100
        || sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate
        || channel_count == 0 || channel_count > kMaximumChannels) {
        throw std::invalid_argument("tape VFX parameters are outside the supported range");
    }
}

float lerp(float from, float to, float amount) noexcept {
    return from + (to - from) * amount;
}

} // namespace

class TapeVfxProcessor::Impl {
  public:
    Impl(TapeVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), parameters_(parameters),
        delay_frames_(
            std::max<std::size_t>(
                4,
                static_cast<std::size_t>(
                    std::ceil(kMaximumDelayMillis * static_cast<float>(sample_rate) / 1000.0F)
                ) + 3
            )
        ),
        delay_line_(delay_frames_ * channel_count_, 0.0F) {
        validate(parameters, sample_rate, channel_count);
        reset_targets();
    }

    void update(TapeVfxParameters parameters) {
        validate(parameters, sample_rate_, channel_count_);
        parameters_ = parameters;
        target_enabled_ = parameters.enabled ? 1.0F : 0.0F;
        target_mix_ = static_cast<float>(parameters.mix_percent) / 100.0F;
        target_saturation_ = static_cast<float>(parameters.saturation_percent) / 100.0F;
        target_motion_ = static_cast<float>(parameters.wow_flutter_percent) / 100.0F;
        target_dropout_ = static_cast<float>(parameters.dropout_percent) / 100.0F;
        smoothing_remaining_ = smoothing_frames_;
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("tape VFX channel layout changed");
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            advance_targets();
            const float slow = std::sin(wow_phase_);
            const float fast = std::sin(flutter_phase_);
            const float motion_delay = motion_ * (1.5F + 5.5F * (0.66F * slow + 0.34F * fast));
            const float dropout_gain = dropout_gain_for_frame();
            const std::size_t base = frame * channel_count_;
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float dry = finite(samples[base + channel]);
                delay_line_[delay_cursor_ * channel_count_ + channel] = dry;
                const float delayed = read_delay(channel, std::max(1.0F, motion_delay));
                const float drive = 1.0F + saturation_ * 5.0F;
                const float normalized = std::tanh(delayed * drive) / std::tanh(drive);
                const float wet = normalized * dropout_gain;
                samples[base + channel] = finite(lerp(dry, wet, enabled_ * mix_));
            }
            delay_cursor_ = (delay_cursor_ + 1) % delay_frames_;
            wow_phase_ = advance_phase(wow_phase_, 0.28F);
            flutter_phase_ = advance_phase(flutter_phase_, 5.7F);
        }
    }

    void reset() noexcept {
        std::fill(delay_line_.begin(), delay_line_.end(), 0.0F);
        delay_cursor_ = 0;
        wow_phase_ = 0.0F;
        flutter_phase_ = 0.0F;
        dropout_state_ = 0x91E10DA5U;
        dropout_remaining_ = 0;
        reset_targets();
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !parameters_.enabled;
    }
    [[nodiscard]] TapeVfxParameters parameters() const noexcept {
        return parameters_;
    }

  private:
    void reset_targets() noexcept {
        enabled_ = target_enabled_ = parameters_.enabled ? 1.0F : 0.0F;
        mix_ = target_mix_ = static_cast<float>(parameters_.mix_percent) / 100.0F;
        saturation_ = target_saturation_ =
            static_cast<float>(parameters_.saturation_percent) / 100.0F;
        motion_ = target_motion_ = static_cast<float>(parameters_.wow_flutter_percent) / 100.0F;
        dropout_ = target_dropout_ = static_cast<float>(parameters_.dropout_percent) / 100.0F;
        smoothing_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        smoothing_remaining_ = 0;
    }

    void advance_targets() noexcept {
        if (smoothing_remaining_ == 0) {
            return;
        }
        const float denominator = static_cast<float>(smoothing_remaining_);
        enabled_ += (target_enabled_ - enabled_) / denominator;
        mix_ += (target_mix_ - mix_) / denominator;
        saturation_ += (target_saturation_ - saturation_) / denominator;
        motion_ += (target_motion_ - motion_) / denominator;
        dropout_ += (target_dropout_ - dropout_) / denominator;
        --smoothing_remaining_;
    }

    [[nodiscard]] float read_delay(std::size_t channel, float delay_frames) const noexcept {
        const float clamped = std::clamp(delay_frames, 1.0F, static_cast<float>(delay_frames_ - 2));
        const std::size_t whole = static_cast<std::size_t>(clamped);
        const float fraction = clamped - static_cast<float>(whole);
        const std::size_t newer = (delay_cursor_ + delay_frames_ - whole) % delay_frames_;
        const std::size_t older = (newer + delay_frames_ - 1) % delay_frames_;
        const float a = delay_line_[newer * channel_count_ + channel];
        const float b = delay_line_[older * channel_count_ + channel];
        return lerp(a, b, fraction);
    }

    [[nodiscard]] float dropout_gain_for_frame() noexcept {
        if (dropout_remaining_ != 0) {
            --dropout_remaining_;
            return 1.0F - 0.88F * dropout_;
        }
        if (dropout_ == 0.0F) {
            return 1.0F;
        }
        dropout_state_ = dropout_state_ * 1664525U + 1013904223U;
        const float random = static_cast<float>(dropout_state_ >> 8U) / 16777216.0F;
        const float likelihood = dropout_ * 0.0008F;
        if (random < likelihood) {
            dropout_remaining_ =
                std::max<std::size_t>(1, static_cast<std::size_t>(sample_rate_ / 500U));
        }
        return 1.0F;
    }

    [[nodiscard]] float advance_phase(float phase, float hertz) const noexcept {
        phase += 2.0F * std::numbers::pi_v<float> * hertz / static_cast<float>(sample_rate_);
        return phase >= 2.0F * std::numbers::pi_v<float> ? phase - 2.0F * std::numbers::pi_v<float>
                                                         : phase;
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    TapeVfxParameters parameters_{};
    std::size_t delay_frames_ = 0;
    std::vector<float> delay_line_;
    std::size_t delay_cursor_ = 0;
    float wow_phase_ = 0.0F;
    float flutter_phase_ = 0.0F;
    std::uint32_t dropout_state_ = 0x91E10DA5U;
    std::size_t dropout_remaining_ = 0;
    std::size_t smoothing_frames_ = 1;
    std::size_t smoothing_remaining_ = 0;
    float enabled_ = 0.0F;
    float mix_ = 0.0F;
    float saturation_ = 0.0F;
    float motion_ = 0.0F;
    float dropout_ = 0.0F;
    float target_enabled_ = 0.0F;
    float target_mix_ = 0.0F;
    float target_saturation_ = 0.0F;
    float target_motion_ = 0.0F;
    float target_dropout_ = 0.0F;
};

TapeVfxProcessor::TapeVfxProcessor(
    TapeVfxParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(parameters, sample_rate, channel_count)) {}

TapeVfxProcessor::~TapeVfxProcessor() = default;
void TapeVfxProcessor::update(TapeVfxParameters parameters) {
    impl_->update(parameters);
}
void TapeVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frames,
    std::size_t channels
) {
    impl_->process_interleaved(samples, frames, channels);
}
void TapeVfxProcessor::reset() {
    impl_->reset();
}
bool TapeVfxProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}
TapeVfxParameters TapeVfxProcessor::parameters() const noexcept {
    return impl_->parameters();
}

} // namespace echo::audio
