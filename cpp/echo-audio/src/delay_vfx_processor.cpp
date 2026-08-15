#include "echo/audio/delay_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <optional>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;
constexpr std::uint16_t kMinimumSlapbackMillis = 30;
constexpr std::uint16_t kMaximumSlapbackMillis = 180;
constexpr std::uint16_t kMinimumEchoMillis = 80;
constexpr std::uint16_t kMaximumEchoMillis = 2000;
constexpr std::uint16_t kMinimumHighCutHertz = 1000;
constexpr std::uint16_t kMaximumHighCutHertz = 20000;
constexpr std::uint8_t kMaximumFeedbackPercent = 90;
constexpr float kMaximumFeedbackSample = 8.0F;
constexpr std::uint16_t kMinimumDuckingAttackMillis = 1;
constexpr std::uint16_t kMaximumDuckingAttackMillis = 200;
constexpr std::uint16_t kMinimumDuckingReleaseMillis = 20;
constexpr std::uint16_t kMaximumDuckingReleaseMillis = 2000;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

bool same(SlapbackAdjustment left, SlapbackAdjustment right) {
    return left.delay_millis == right.delay_millis && left.mix_percent == right.mix_percent
           && left.high_cut_hertz == right.high_cut_hertz;
}

bool same(EchoAdjustment left, EchoAdjustment right) {
    return left.delay_millis == right.delay_millis
           && left.feedback_percent == right.feedback_percent
           && left.mix_percent == right.mix_percent && left.high_cut_hertz == right.high_cut_hertz
           && left.stereo_crossfeed_percent == right.stereo_crossfeed_percent;
}

bool same(DelayVfxAdjustment left, DelayVfxAdjustment right) {
    return left.character == right.character && left.enabled == right.enabled
           && same(left.slapback, right.slapback) && same(left.echo, right.echo)
           && left.ducking.enabled == right.ducking.enabled
           && left.ducking.amount_percent == right.ducking.amount_percent
           && left.ducking.attack_millis == right.ducking.attack_millis
           && left.ducking.release_millis == right.ducking.release_millis;
}

void validate(DelayVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) {
    const bool character_valid = adjustment.character == DelayVfxCharacter::Slapback
                                 || adjustment.character == DelayVfxCharacter::Echo;
    const bool slapback_valid = adjustment.slapback.delay_millis >= kMinimumSlapbackMillis
                                && adjustment.slapback.delay_millis <= kMaximumSlapbackMillis
                                && adjustment.slapback.mix_percent <= 100
                                && adjustment.slapback.high_cut_hertz >= kMinimumHighCutHertz
                                && adjustment.slapback.high_cut_hertz <= kMaximumHighCutHertz;
    const bool echo_valid = adjustment.echo.delay_millis >= kMinimumEchoMillis
                            && adjustment.echo.delay_millis <= kMaximumEchoMillis
                            && adjustment.echo.feedback_percent <= kMaximumFeedbackPercent
                            && adjustment.echo.mix_percent <= 100
                            && adjustment.echo.high_cut_hertz >= kMinimumHighCutHertz
                            && adjustment.echo.high_cut_hertz <= kMaximumHighCutHertz
                            && adjustment.echo.stereo_crossfeed_percent <= 100;
    const bool ducking_valid = adjustment.ducking.amount_percent <= 100
                               && adjustment.ducking.attack_millis >= kMinimumDuckingAttackMillis
                               && adjustment.ducking.attack_millis <= kMaximumDuckingAttackMillis
                               && adjustment.ducking.release_millis >= kMinimumDuckingReleaseMillis
                               && adjustment.ducking.release_millis <= kMaximumDuckingReleaseMillis;
    if (!character_valid || !slapback_valid || !echo_valid || !ducking_valid
        || sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate
        || channel_count == 0 || channel_count > 2) {
        throw std::invalid_argument("delay VFX parameters are outside the supported range");
    }
}

} // namespace

class DelayVfxProcessor::Impl {
  public:
    Impl(DelayVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count) {
        validate(adjustment, sample_rate_, channel_count_);
        const double maximum_frames =
            static_cast<double>(kMaximumEchoMillis) * static_cast<double>(sample_rate_) / 1000.0;
        delay_capacity_frames_ = static_cast<std::size_t>(std::ceil(maximum_frames)) + 2;
        delay_.assign(delay_capacity_frames_ * channel_count_, 0.0F);
        transition_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        authored_ = adjustment;
        active_ = prepare(adjustment);
    }

    void update(DelayVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        if (transitioning_) {
            pending_ = adjustment;
        } else if (!same(active_.authored, adjustment)) {
            begin_transition(adjustment);
        }
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("delay VFX channel layout changed");
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            const std::array<float, 2> input{
                finite(samples[base]),
                channel_count_ == 1 ? finite(samples[base]) : finite(samples[base + 1]),
            };
            const Render active = render(active_, active_state_, input);
            Render output = active;
            float transition = 0.0F;
            if (transitioning_) {
                const Render next = render(next_, next_state_, input);
                transition = std::min(
                    1.0F,
                    static_cast<float>(transition_frame_ + 1)
                        / static_cast<float>(transition_frames_)
                );
                for (std::size_t channel = 0; channel < 2; ++channel) {
                    output.samples[channel] =
                        active.samples[channel]
                        + transition * (next.samples[channel] - active.samples[channel]);
                    output.feedback[channel] =
                        active.feedback[channel]
                        + transition * (next.feedback[channel] - active.feedback[channel]);
                }
            }

            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float written = std::clamp(
                    finite(input[channel] + output.feedback[channel]),
                    -kMaximumFeedbackSample,
                    kMaximumFeedbackSample
                );
                delay_[delay_cursor_ * channel_count_ + channel] = written;
                samples[base + channel] = finite(output.samples[channel]);
            }
            delay_cursor_ = (delay_cursor_ + 1) % delay_capacity_frames_;

            if (transitioning_) {
                ++transition_frame_;
                if (transition_frame_ >= transition_frames_) {
                    complete_transition();
                }
            }
        }
    }

    void reset() {
        std::fill(delay_.begin(), delay_.end(), 0.0F);
        delay_cursor_ = 0;
        active_ = prepare(authored_);
        active_state_ = {};
        next_state_ = {};
        pending_.reset();
        transitioning_ = false;
        transition_frame_ = 0;
    }

    [[nodiscard]] bool is_bypassed() const {
        return !authored_.enabled;
    }

  private:
    struct Prepared {
        DelayVfxAdjustment authored;
        double delay_frames = 1.0;
        float mix = 0.0F;
        float feedback = 0.0F;
        float crossfeed = 0.0F;
        float lowpass_alpha = 1.0F;
        float ducking_amount = 0.0F;
        float ducking_attack_alpha = 0.0F;
        float ducking_release_alpha = 0.0F;
    };

    struct PathState {
        std::array<float, 2> lowpass{};
        float ducking_envelope = 0.0F;
    };

    struct Render {
        std::array<float, 2> samples{};
        std::array<float, 2> feedback{};
    };

    [[nodiscard]] Prepared prepare(DelayVfxAdjustment adjustment) const {
        Prepared prepared{.authored = adjustment};
        std::uint16_t delay_millis = 0;
        std::uint16_t high_cut_hertz = 0;
        if (adjustment.character == DelayVfxCharacter::Slapback) {
            delay_millis = adjustment.slapback.delay_millis;
            high_cut_hertz = adjustment.slapback.high_cut_hertz;
            prepared.mix = static_cast<float>(adjustment.slapback.mix_percent) / 100.0F;
        } else {
            delay_millis = adjustment.echo.delay_millis;
            high_cut_hertz = adjustment.echo.high_cut_hertz;
            prepared.mix = static_cast<float>(adjustment.echo.mix_percent) / 100.0F;
            prepared.feedback = static_cast<float>(adjustment.echo.feedback_percent) / 100.0F;
            prepared.crossfeed =
                static_cast<float>(adjustment.echo.stereo_crossfeed_percent) / 100.0F;
        }
        prepared.delay_frames =
            static_cast<double>(delay_millis) * static_cast<double>(sample_rate_) / 1000.0;
        const double cutoff =
            std::min(static_cast<double>(high_cut_hertz), 0.45 * static_cast<double>(sample_rate_));
        prepared.lowpass_alpha =
            static_cast<float>(1.0 - std::exp(-2.0 * std::numbers::pi * cutoff / sample_rate_));
        prepared.ducking_amount =
            adjustment.ducking.enabled
                ? static_cast<float>(adjustment.ducking.amount_percent) / 100.0F
                : 0.0F;
        const auto alpha_for = [this](std::uint16_t millis) {
            const float frames =
                static_cast<float>(sample_rate_) * static_cast<float>(millis) / 1000.0F;
            return std::exp(-1.0F / std::max(1.0F, frames));
        };
        prepared.ducking_attack_alpha = alpha_for(adjustment.ducking.attack_millis);
        prepared.ducking_release_alpha = alpha_for(adjustment.ducking.release_millis);
        return prepared;
    }

    [[nodiscard]] float read_delay(std::size_t channel, double delay_frames) const {
        const auto whole = static_cast<std::size_t>(std::floor(delay_frames));
        const float fraction = static_cast<float>(delay_frames - static_cast<double>(whole));
        const std::size_t recent_frame =
            (delay_cursor_ + delay_capacity_frames_ - whole % delay_capacity_frames_)
            % delay_capacity_frames_;
        const std::size_t older_frame =
            (recent_frame + delay_capacity_frames_ - 1) % delay_capacity_frames_;
        const float recent = delay_[recent_frame * channel_count_ + channel];
        const float older = delay_[older_frame * channel_count_ + channel];
        return recent + fraction * (older - recent);
    }

    [[nodiscard]] Render
    render(const Prepared& path, PathState& state, const std::array<float, 2>& input) const {
        Render rendered;
        std::array<float, 2> wet{};
        const float target =
            std::clamp(0.5F * (std::abs(input[0]) + std::abs(input[1])), 0.0F, 1.0F);
        const float alpha = target > state.ducking_envelope ? path.ducking_attack_alpha
                                                            : path.ducking_release_alpha;
        state.ducking_envelope = target + alpha * (state.ducking_envelope - target);
        const float wet_mix = path.mix * (1.0F - path.ducking_amount * state.ducking_envelope);
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const float delayed = read_delay(channel, path.delay_frames);
            state.lowpass[channel] += path.lowpass_alpha * (delayed - state.lowpass[channel]);
            wet[channel] = finite(state.lowpass[channel]);
            rendered.samples[channel] =
                path.authored.enabled ? input[channel] + wet_mix * (wet[channel] - input[channel])
                                      : input[channel];
        }
        if (channel_count_ == 1) {
            wet[1] = wet[0];
            rendered.samples[1] = rendered.samples[0];
        }
        if (path.authored.character == DelayVfxCharacter::Echo) {
            const float same_channel = path.feedback * (1.0F - path.crossfeed);
            const float cross_channel = path.feedback * path.crossfeed;
            rendered.feedback[0] = same_channel * wet[0] + cross_channel * wet[1];
            rendered.feedback[1] = same_channel * wet[1] + cross_channel * wet[0];
        }
        return rendered;
    }

    void begin_transition(DelayVfxAdjustment adjustment) {
        next_ = prepare(adjustment);
        next_state_ = {};
        transition_frame_ = 0;
        transitioning_ = true;
    }

    void complete_transition() {
        active_ = next_;
        active_state_ = next_state_;
        transitioning_ = false;
        transition_frame_ = 0;
        if (pending_.has_value()) {
            const DelayVfxAdjustment pending = *pending_;
            pending_.reset();
            if (!same(active_.authored, pending)) {
                begin_transition(pending);
            }
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::vector<float> delay_;
    std::size_t delay_capacity_frames_ = 0;
    std::size_t delay_cursor_ = 0;
    DelayVfxAdjustment authored_;
    Prepared active_;
    Prepared next_;
    PathState active_state_;
    PathState next_state_;
    std::optional<DelayVfxAdjustment> pending_;
    std::size_t transition_frame_ = 0;
    std::size_t transition_frames_ = 1;
    bool transitioning_ = false;
};

DelayVfxProcessor::DelayVfxProcessor(
    DelayVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

DelayVfxProcessor::~DelayVfxProcessor() = default;

void DelayVfxProcessor::update(DelayVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void DelayVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void DelayVfxProcessor::reset() {
    impl_->reset();
}

bool DelayVfxProcessor::is_bypassed() const {
    return impl_->is_bypassed();
}

} // namespace echo::audio
