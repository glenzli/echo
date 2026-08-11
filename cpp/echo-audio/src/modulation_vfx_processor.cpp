#include "echo/audio/modulation_vfx_processor.hpp"

#include "modulated_delay_vfx.hpp"
#include "phaser_vfx.hpp"
#include "tremolo_vfx.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <optional>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;

float finite(float sample) {
    return std::isfinite(sample) ? sample : 0.0F;
}

float smoothing_coefficient(std::uint32_t sample_rate) {
    return static_cast<float>(1.0 - std::exp(-1.0 / (0.02 * static_cast<double>(sample_rate))));
}

void validate(
    ModulationVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    const bool character_valid = adjustment.character == ModulationVfxCharacter::Chorus
                                 || adjustment.character == ModulationVfxCharacter::Flanger
                                 || adjustment.character == ModulationVfxCharacter::Phaser
                                 || adjustment.character == ModulationVfxCharacter::Tremolo;
    const std::uint32_t chorus_delay =
        static_cast<std::uint32_t>(adjustment.chorus.minimum_delay_microseconds)
        + static_cast<std::uint32_t>(adjustment.chorus.sweep_microseconds);
    const bool chorus_valid =
        adjustment.chorus.mix_percent <= 100 && adjustment.chorus.rate_millihertz >= 50
        && adjustment.chorus.rate_millihertz <= 5000
        && adjustment.chorus.minimum_delay_microseconds >= 5000
        && adjustment.chorus.minimum_delay_microseconds <= 25000
        && adjustment.chorus.sweep_microseconds >= 500
        && adjustment.chorus.sweep_microseconds <= 20000 && chorus_delay <= 50000
        && adjustment.chorus.stereo_phase_degrees <= 180;
    const std::uint32_t flanger_delay =
        static_cast<std::uint32_t>(adjustment.flanger.minimum_delay_microseconds)
        + static_cast<std::uint32_t>(adjustment.flanger.sweep_microseconds);
    const int flanger_feedback = static_cast<int>(adjustment.flanger.feedback_percent);
    const bool flanger_valid =
        adjustment.flanger.mix_percent <= 100 && adjustment.flanger.rate_millihertz >= 50
        && adjustment.flanger.rate_millihertz <= 10000
        && adjustment.flanger.minimum_delay_microseconds >= 100
        && adjustment.flanger.minimum_delay_microseconds <= 5000
        && adjustment.flanger.sweep_microseconds >= 100
        && adjustment.flanger.sweep_microseconds <= 10000 && flanger_delay <= 15000
        && flanger_feedback >= -90 && flanger_feedback <= 90
        && adjustment.flanger.stereo_phase_degrees <= 180;
    const int phaser_feedback = static_cast<int>(adjustment.phaser.feedback_percent);
    const bool phaser_valid =
        adjustment.phaser.mix_percent <= 100 && adjustment.phaser.rate_millihertz >= 50
        && adjustment.phaser.rate_millihertz <= 10000 && adjustment.phaser.sweep_low_hertz >= 50
        && adjustment.phaser.sweep_low_hertz <= 4000 && adjustment.phaser.sweep_high_hertz >= 500
        && adjustment.phaser.sweep_high_hertz <= 12000
        && adjustment.phaser.sweep_low_hertz < adjustment.phaser.sweep_high_hertz
        && phaser_feedback >= -90 && phaser_feedback <= 90
        && adjustment.phaser.stereo_phase_degrees <= 180;
    const bool tremolo_valid = adjustment.tremolo.rate_millihertz >= 100
                               && adjustment.tremolo.rate_millihertz <= 20000
                               && adjustment.tremolo.depth_percent <= 100
                               && adjustment.tremolo.stereo_phase_degrees <= 180;
    if (!character_valid || !chorus_valid || !flanger_valid || !phaser_valid || !tremolo_valid
        || sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate
        || channel_count == 0 || channel_count > 2) {
        throw std::invalid_argument("modulation VFX parameters are outside the supported range");
    }
}

} // namespace

class ModulationVfxProcessor::Impl {
  public:
    Impl(ModulationVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        enabled_smoothing_(smoothing_coefficient(sample_rate)),
        chorus_(adjustment.chorus, sample_rate, channel_count),
        flanger_(adjustment.flanger, sample_rate, channel_count),
        phaser_(adjustment.phaser, sample_rate, channel_count),
        tremolo_(adjustment.tremolo, sample_rate, channel_count) {
        validate(adjustment, sample_rate_, channel_count_);
        transition_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        authored_ = adjustment;
        active_character_ = adjustment.character;
        enabled_ = adjustment.enabled ? 1.0F : 0.0F;
        target_enabled_ = enabled_;
    }

    void update(ModulationVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        if (transitioning_) {
            pending_ = adjustment;
        } else {
            apply_update(adjustment);
        }
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("modulation VFX channel layout changed");
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            const std::array<float, 2> input{
                finite(samples[base]),
                channel_count_ == 1 ? finite(samples[base]) : finite(samples[base + 1]),
            };
            const auto active = process_character(active_character_, input);
            std::array<float, 2> effected = active;
            if (transitioning_) {
                const auto next = process_character(next_character_, input);
                const float progress = std::min(
                    1.0F,
                    static_cast<float>(transition_frame_ + 1)
                        / static_cast<float>(transition_frames_)
                );
                for (std::size_t channel = 0; channel < 2; ++channel) {
                    effected[channel] =
                        active[channel] + progress * (next[channel] - active[channel]);
                }
            }
            enabled_ += enabled_smoothing_ * (target_enabled_ - enabled_);
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                samples[base + channel] =
                    enabled_ == 0.0F
                        ? input[channel]
                        : finite(input[channel] + enabled_ * (effected[channel] - input[channel]));
            }
            if (transitioning_) {
                ++transition_frame_;
                if (transition_frame_ >= transition_frames_) {
                    complete_transition();
                }
            }
        }
    }

    void reset() {
        active_character_ = authored_.character;
        next_character_ = authored_.character;
        update_engines(authored_);
        chorus_.reset();
        flanger_.reset();
        phaser_.reset();
        tremolo_.reset();
        enabled_ = authored_.enabled ? 1.0F : 0.0F;
        target_enabled_ = enabled_;
        pending_.reset();
        transitioning_ = false;
        transition_frame_ = 0;
    }

    [[nodiscard]] bool is_bypassed() const {
        return !authored_.enabled;
    }

  private:
    void update_engines(ModulationVfxAdjustment adjustment) {
        chorus_.update(adjustment.chorus);
        flanger_.update(adjustment.flanger);
        phaser_.update(adjustment.phaser);
        tremolo_.update(adjustment.tremolo);
    }

    void reset_character(ModulationVfxCharacter character) {
        switch (character) {
        case ModulationVfxCharacter::Chorus:
            chorus_.reset();
            break;
        case ModulationVfxCharacter::Flanger:
            flanger_.reset();
            break;
        case ModulationVfxCharacter::Phaser:
            phaser_.reset();
            break;
        case ModulationVfxCharacter::Tremolo:
            tremolo_.reset();
            break;
        }
    }

    [[nodiscard]] std::array<float, 2>
    process_character(ModulationVfxCharacter character, const std::array<float, 2>& input) {
        switch (character) {
        case ModulationVfxCharacter::Chorus:
            return chorus_.process(input);
        case ModulationVfxCharacter::Flanger:
            return flanger_.process(input);
        case ModulationVfxCharacter::Phaser:
            return phaser_.process(input);
        case ModulationVfxCharacter::Tremolo:
            return tremolo_.process(input);
        }
        return input;
    }

    void apply_update(ModulationVfxAdjustment adjustment) {
        update_engines(adjustment);
        target_enabled_ = adjustment.enabled ? 1.0F : 0.0F;
        if (adjustment.character != active_character_) {
            next_character_ = adjustment.character;
            reset_character(next_character_);
            transition_frame_ = 0;
            transitioning_ = true;
        }
    }

    void complete_transition() {
        active_character_ = next_character_;
        transitioning_ = false;
        transition_frame_ = 0;
        if (pending_.has_value()) {
            const ModulationVfxAdjustment pending = *pending_;
            pending_.reset();
            apply_update(pending);
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    float enabled_smoothing_ = 1.0F;
    detail::ChorusVfx chorus_;
    detail::FlangerVfx flanger_;
    detail::PhaserVfx phaser_;
    detail::TremoloVfx tremolo_;
    float enabled_ = 0.0F;
    float target_enabled_ = 0.0F;
    ModulationVfxAdjustment authored_;
    ModulationVfxCharacter active_character_ = ModulationVfxCharacter::Chorus;
    ModulationVfxCharacter next_character_ = ModulationVfxCharacter::Chorus;
    std::optional<ModulationVfxAdjustment> pending_;
    std::size_t transition_frame_ = 0;
    std::size_t transition_frames_ = 1;
    bool transitioning_ = false;
};

ModulationVfxProcessor::ModulationVfxProcessor(
    ModulationVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

ModulationVfxProcessor::~ModulationVfxProcessor() = default;

void ModulationVfxProcessor::update(ModulationVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void ModulationVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void ModulationVfxProcessor::reset() {
    impl_->reset();
}

bool ModulationVfxProcessor::is_bypassed() const {
    return impl_->is_bypassed();
}

} // namespace echo::audio
