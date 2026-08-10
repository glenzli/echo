#include "echo/audio/effect_processing_chain.hpp"

#include "echo/audio/adaptive_noise_reducer.hpp"
#include "echo/audio/algorithmic_reverb.hpp"
#include "echo/audio/de_click_processor.hpp"
#include "echo/audio/de_esser.hpp"
#include "echo/audio/de_hum_filter.hpp"
#include "echo/audio/dynamics_processor.hpp"
#include "echo/audio/parametric_equalizer.hpp"

#include <algorithm>
#include <cstring>
#include <limits>
#include <stdexcept>

namespace echo::audio {

class EffectProcessingChain::Impl {
  public:
    Impl(
        const PreparedAdjustment& adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    ) :
        sample_rate_(sample_rate), channel_count_(channel_count), nodes_(adjustment.effect_chain()),
        node_count_(adjustment.effect_chain_count()),
        noise_reducer_(adjustment.restoration().noise_reduction, sample_rate),
        de_esser_(adjustment.restoration().de_esser, sample_rate, channel_count),
        de_hum_(
            {
                .enabled = adjustment.de_hum().enabled,
                .fundamental_hertz = adjustment.de_hum().fundamental_hertz,
                .harmonic_count = adjustment.de_hum().harmonic_count,
                .quality_tenths = adjustment.de_hum().quality_tenths,
                .depth_centibels = adjustment.de_hum().depth_centibels,
            },
            sample_rate,
            channel_count
        ),
        de_click_(
            {
                .enabled = adjustment.de_click().enabled,
                .sensitivity_percent = adjustment.de_click().sensitivity_percent,
                .maximum_click_microseconds = adjustment.de_click().maximum_click_microseconds,
                .repair_percent = adjustment.de_click().repair_percent,
            },
            sample_rate,
            channel_count
        ),
        equalizer_(adjustment.equalizer(), sample_rate, channel_count),
        dynamics_(adjustment.compressor(), sample_rate),
        reverb_(adjustment.reverb(), sample_rate, channel_count),
        restoration_enabled_(adjustment.restoration().enabled) {
        if (sample_rate_ == 0 || channel_count_ == 0) {
            throw std::invalid_argument("effect chain requires a valid audio layout");
        }
        for (std::size_t index = 0; index + 1 < node_count_; ++index) {
            if (nodes_[index] == EffectNodeKind::DeClick) {
                latency_frames_ += de_click_.latency_frames();
            }
        }
        reset_compensation();
    }

    std::size_t
    process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        validate_buffer(samples, frame_count, channel_count);
        if (finishing_) {
            throw std::logic_error("effect chain cannot accept input after finishing begins");
        }
        if (frame_count > 0
            && pending_output_frames_ > std::numeric_limits<std::size_t>::max() - frame_count) {
            throw std::overflow_error("effect chain pending frame count overflowed");
        }
        pending_output_frames_ += frame_count;
        process_raw(samples, frame_count);
        return compact_output(samples, frame_count);
    }

    std::size_t
    finish_interleaved(float* samples, std::size_t capacity_frames, std::size_t channel_count) {
        validate_buffer(samples, capacity_frames, channel_count);
        if (!finishing_) {
            finishing_ = true;
            drain_input_frames_ = latency_frames_;
        }
        if (pending_output_frames_ == 0 || capacity_frames == 0) {
            return 0;
        }
        while (pending_output_frames_ > 0) {
            const std::size_t input_frames = std::min(capacity_frames, drain_input_frames_);
            if (input_frames == 0) {
                throw std::logic_error("effect chain latency drain ended with pending output");
            }
            std::fill_n(samples, input_frames * channel_count_, 0.0F);
            process_raw(samples, input_frames);
            drain_input_frames_ -= input_frames;
            const std::size_t produced = compact_output(samples, input_frames);
            if (produced > 0) {
                return produced;
            }
        }
        return 0;
    }

    void reset() {
        noise_reducer_.reset();
        de_esser_.reset();
        de_hum_.reset();
        de_click_.reset();
        equalizer_.reset();
        dynamics_.reset();
        reverb_.reset();
        reset_compensation();
    }

    void update_restoration(RestorationAdjustment adjustment) {
        noise_reducer_.update(adjustment.noise_reduction);
        de_esser_.update(adjustment.de_esser);
        restoration_enabled_ = adjustment.enabled;
    }

    void update_de_hum(DeHumAdjustment adjustment) {
        de_hum_.update({
            .enabled = adjustment.enabled,
            .fundamental_hertz = adjustment.fundamental_hertz,
            .harmonic_count = adjustment.harmonic_count,
            .quality_tenths = adjustment.quality_tenths,
            .depth_centibels = adjustment.depth_centibels,
        });
    }

    void update_de_click(DeClickAdjustment adjustment) {
        de_click_.update({
            .enabled = adjustment.enabled,
            .sensitivity_percent = adjustment.sensitivity_percent,
            .maximum_click_microseconds = adjustment.maximum_click_microseconds,
            .repair_percent = adjustment.repair_percent,
        });
    }

    void update_equalizer(ParametricEqualizerAdjustment adjustment) {
        equalizer_.transition_to(adjustment);
    }

    void update_compressor(CompressorAdjustment adjustment) {
        dynamics_.update(adjustment);
    }

    void update_reverb(ReverbAdjustment adjustment) {
        reverb_.update(adjustment);
    }

    [[nodiscard]] std::size_t latency_frames() const {
        return latency_frames_;
    }

    [[nodiscard]] std::size_t pending_output_frames() const {
        return pending_output_frames_;
    }

    [[nodiscard]] float gain_reduction_decibels() const {
        return dynamics_.gain_reduction_decibels();
    }

  private:
    void validate_buffer(
        const float* samples,
        std::size_t frame_count,
        std::size_t channel_count
    ) const {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("effect chain channel layout changed");
        }
    }

    void process_raw(float* samples, std::size_t frame_count) {
        for (std::size_t node_index = 0; node_index + 1 < node_count_; ++node_index) {
            switch (nodes_[node_index]) {
            case EffectNodeKind::Restoration:
                if (restoration_enabled_) {
                    noise_reducer_.process_interleaved(samples, frame_count, channel_count_);
                    de_esser_.process_interleaved(samples, frame_count, channel_count_);
                }
                break;
            case EffectNodeKind::DeHum:
                de_hum_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::DeClick:
                de_click_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::Equalizer:
                for (std::size_t frame = 0; frame < frame_count; ++frame) {
                    for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                        const std::size_t sample_index = frame * channel_count_ + channel;
                        samples[sample_index] =
                            equalizer_.process_sample(samples[sample_index], channel);
                    }
                }
                break;
            case EffectNodeKind::Dynamics:
                dynamics_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::Space:
                reverb_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::Master:
                break;
            }
        }
    }

    std::size_t compact_output(float* samples, std::size_t processed_frames) {
        const std::size_t discarded = std::min(front_discard_frames_, processed_frames);
        front_discard_frames_ -= discarded;
        const std::size_t available = processed_frames - discarded;
        const std::size_t produced = std::min(available, pending_output_frames_);
        if (produced > 0 && discarded > 0) {
            std::memmove(
                samples,
                samples + discarded * channel_count_,
                produced * channel_count_ * sizeof(float)
            );
        }
        pending_output_frames_ -= produced;
        return produced;
    }

    void reset_compensation() {
        front_discard_frames_ = latency_frames_;
        pending_output_frames_ = 0;
        drain_input_frames_ = 0;
        finishing_ = false;
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::array<EffectNodeKind, kEffectNodeCount> nodes_{};
    std::size_t node_count_ = 0;
    AdaptiveNoiseReducer noise_reducer_;
    DeEsser de_esser_;
    DeHumFilter de_hum_;
    DeClickProcessor de_click_;
    ParametricEqualizer equalizer_;
    DynamicsProcessor dynamics_;
    AlgorithmicReverb reverb_;
    bool restoration_enabled_ = true;
    std::size_t latency_frames_ = 0;
    std::size_t front_discard_frames_ = 0;
    std::size_t pending_output_frames_ = 0;
    std::size_t drain_input_frames_ = 0;
    bool finishing_ = false;
};

EffectProcessingChain::EffectProcessingChain(
    const PreparedAdjustment& adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

EffectProcessingChain::~EffectProcessingChain() = default;

std::size_t EffectProcessingChain::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    return impl_->process_interleaved(samples, frame_count, channel_count);
}

std::size_t EffectProcessingChain::finish_interleaved(
    float* samples,
    std::size_t capacity_frames,
    std::size_t channel_count
) {
    return impl_->finish_interleaved(samples, capacity_frames, channel_count);
}

void EffectProcessingChain::reset() {
    impl_->reset();
}

void EffectProcessingChain::update_restoration(RestorationAdjustment adjustment) {
    impl_->update_restoration(adjustment);
}

void EffectProcessingChain::update_de_hum(DeHumAdjustment adjustment) {
    impl_->update_de_hum(adjustment);
}

void EffectProcessingChain::update_de_click(DeClickAdjustment adjustment) {
    impl_->update_de_click(adjustment);
}

void EffectProcessingChain::update_equalizer(ParametricEqualizerAdjustment adjustment) {
    impl_->update_equalizer(adjustment);
}

void EffectProcessingChain::update_compressor(CompressorAdjustment adjustment) {
    impl_->update_compressor(adjustment);
}

void EffectProcessingChain::update_reverb(ReverbAdjustment adjustment) {
    impl_->update_reverb(adjustment);
}

void EffectProcessingChain::validate_restoration(
    RestorationAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const AdaptiveNoiseReducer noise(adjustment.noise_reduction, sample_rate);
    [[maybe_unused]] const DeEsser de_esser(adjustment.de_esser, sample_rate, channel_count);
}

void EffectProcessingChain::validate_de_hum(
    DeHumAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const DeHumFilter filter(
        {
            .enabled = adjustment.enabled,
            .fundamental_hertz = adjustment.fundamental_hertz,
            .harmonic_count = adjustment.harmonic_count,
            .quality_tenths = adjustment.quality_tenths,
            .depth_centibels = adjustment.depth_centibels,
        },
        sample_rate,
        channel_count
    );
}

void EffectProcessingChain::validate_de_click(
    DeClickAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const DeClickProcessor processor(
        {
            .enabled = adjustment.enabled,
            .sensitivity_percent = adjustment.sensitivity_percent,
            .maximum_click_microseconds = adjustment.maximum_click_microseconds,
            .repair_percent = adjustment.repair_percent,
        },
        sample_rate,
        channel_count
    );
}

void EffectProcessingChain::validate_equalizer(
    ParametricEqualizerAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const ParametricEqualizer equalizer(adjustment, sample_rate, channel_count);
}

void EffectProcessingChain::validate_compressor(
    CompressorAdjustment adjustment,
    std::uint32_t sample_rate
) {
    [[maybe_unused]] const DynamicsProcessor processor(adjustment, sample_rate);
}

void EffectProcessingChain::validate_reverb(
    ReverbAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const AlgorithmicReverb reverb(adjustment, sample_rate, channel_count);
}

std::size_t EffectProcessingChain::latency_frames() const {
    return impl_->latency_frames();
}

std::size_t EffectProcessingChain::pending_output_frames() const {
    return impl_->pending_output_frames();
}

float EffectProcessingChain::gain_reduction_decibels() const {
    return impl_->gain_reduction_decibels();
}

} // namespace echo::audio
