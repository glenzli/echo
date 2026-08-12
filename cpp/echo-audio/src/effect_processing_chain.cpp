#include "echo/audio/effect_processing_chain.hpp"

#include "echo/audio/adaptive_noise_reducer.hpp"
#include "echo/audio/algorithmic_reverb.hpp"
#include "echo/audio/channel_repair_processor.hpp"
#include "echo/audio/de_click_processor.hpp"
#include "echo/audio/de_esser.hpp"
#include "echo/audio/de_hum_filter.hpp"
#include "echo/audio/de_plosive_processor.hpp"
#include "echo/audio/delay_vfx_processor.hpp"
#include "echo/audio/digital_degrade_vfx_processor.hpp"
#include "echo/audio/drive_vfx_processor.hpp"
#include "echo/audio/dynamics_processor.hpp"
#include "echo/audio/modulation_vfx_processor.hpp"
#include "echo/audio/parametric_equalizer.hpp"
#include "echo/audio/rotary_vfx_processor.hpp"
#include "echo/audio/scene_vfx_processor.hpp"
#include "echo/audio/transform_vfx_processor.hpp"

#include <algorithm>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::size_t kMaximumProcessingFrames = 4096;

} // namespace

class EffectProcessingChain::Impl {
  public:
    Impl(
        const PreparedAdjustment& adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count,
        const EffectMaskPlan* mask_plan
    ) :
        sample_rate_(sample_rate), channel_count_(channel_count), nodes_(adjustment.effect_chain()),
        node_count_(adjustment.effect_chain_count()),
        de_plosive_(adjustment.restoration().de_plosive, sample_rate, channel_count),
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
        channel_repair_(adjustment.channel_repair(), sample_rate, channel_count),
        equalizer_(adjustment.equalizer(), sample_rate, channel_count),
        dynamics_(adjustment.compressor(), sample_rate),
        reverb_(adjustment.reverb(), sample_rate, channel_count),
        scene_vfx_(adjustment.creative_vfx().scene, sample_rate, channel_count),
        delay_vfx_(adjustment.creative_vfx().delay, sample_rate, channel_count),
        modulation_vfx_(adjustment.creative_vfx().modulation, sample_rate, channel_count),
        transform_vfx_(adjustment.creative_vfx().transform, sample_rate, channel_count),
        digital_degrade_vfx_(adjustment.creative_vfx().digital_degrade, sample_rate, channel_count),
        drive_vfx_(adjustment.creative_vfx().drive, sample_rate, channel_count),
        rotary_vfx_(adjustment.creative_vfx().rotary, sample_rate, channel_count),
        restoration_enabled_(adjustment.restoration().enabled), mask_plan_(mask_plan),
        dry_samples_(kMaximumProcessingFrames * channel_count, 0.0F) {
        if (sample_rate_ == 0 || channel_count_ == 0) {
            throw std::invalid_argument("effect chain requires a valid audio layout");
        }
        for (std::size_t index = 0; index + 1 < node_count_; ++index) {
            const std::size_t node_latency = latency_frames_for(nodes_[index]);
            latency_frames_ += node_latency;
            source_delays_[static_cast<std::size_t>(nodes_[index])].assign(
                node_latency,
                kNoSourceFrame
            );
            has_local_masks_ =
                has_local_masks_
                || (mask_plan_ != nullptr && mask_plan_->is_locally_masked(nodes_[index]));
        }
        reset_compensation();
    }

    std::size_t
    process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        return process_interleaved(samples, nullptr, frame_count, channel_count);
    }

    std::size_t process_interleaved(
        float* samples,
        std::uint64_t* source_frames,
        std::size_t frame_count,
        std::size_t channel_count
    ) {
        validate_buffer(samples, frame_count, channel_count);
        if (finishing_) {
            throw std::logic_error("effect chain cannot accept input after finishing begins");
        }
        if (frame_count > 0
            && pending_output_frames_ > std::numeric_limits<std::size_t>::max() - frame_count) {
            throw std::overflow_error("effect chain pending frame count overflowed");
        }
        pending_output_frames_ += frame_count;
        process_raw(samples, source_frames, frame_count);
        return compact_output(samples, source_frames, frame_count);
    }

    std::size_t
    finish_interleaved(float* samples, std::size_t capacity_frames, std::size_t channel_count) {
        return finish_interleaved(samples, nullptr, capacity_frames, channel_count);
    }

    std::size_t finish_interleaved(
        float* samples,
        std::uint64_t* source_frames,
        std::size_t capacity_frames,
        std::size_t channel_count
    ) {
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
            if (source_frames != nullptr) {
                std::fill_n(source_frames, input_frames, kNoSourceFrame);
            }
            process_raw(samples, source_frames, input_frames);
            drain_input_frames_ -= input_frames;
            const std::size_t produced = compact_output(samples, source_frames, input_frames);
            if (produced > 0) {
                return produced;
            }
        }
        return 0;
    }

    void reset() {
        de_plosive_.reset();
        noise_reducer_.reset();
        de_esser_.reset();
        de_hum_.reset();
        de_click_.reset();
        channel_repair_.reset();
        equalizer_.reset();
        dynamics_.reset();
        reverb_.reset();
        scene_vfx_.reset();
        delay_vfx_.reset();
        modulation_vfx_.reset();
        transform_vfx_.reset();
        digital_degrade_vfx_.reset();
        drive_vfx_.reset();
        rotary_vfx_.reset();
        reset_compensation();
    }

    void update_restoration(RestorationAdjustment adjustment) {
        de_plosive_.update(adjustment.de_plosive);
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

    void update_channel_repair(ChannelRepairAdjustment adjustment) {
        channel_repair_.update(adjustment);
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

    void update_scene_vfx(SceneVfxAdjustment adjustment) {
        scene_vfx_.update(adjustment);
    }

    void update_delay_vfx(DelayVfxAdjustment adjustment) {
        delay_vfx_.update(adjustment);
    }

    void update_modulation_vfx(ModulationVfxAdjustment adjustment) {
        modulation_vfx_.update(adjustment);
    }

    void update_transform_vfx(TransformVfxAdjustment adjustment) {
        transform_vfx_.update(adjustment);
    }

    void update_digital_degrade_vfx(DigitalDegradeVfxAdjustment adjustment) {
        digital_degrade_vfx_.update(adjustment);
    }

    void update_drive_vfx(DriveVfxAdjustment adjustment) {
        drive_vfx_.update(adjustment);
    }

    void update_rotary_vfx(RotaryVfxAdjustment adjustment) {
        rotary_vfx_.update(adjustment);
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
    [[nodiscard]] std::size_t latency_frames_for(EffectNodeKind node) const {
        switch (node) {
        case EffectNodeKind::DeClick:
            return de_click_.latency_frames();
        case EffectNodeKind::TransformVfx:
            return transform_vfx_.latency_frames();
        case EffectNodeKind::DriveVfx:
            return drive_vfx_.latency_frames();
        case EffectNodeKind::Restoration:
        case EffectNodeKind::Equalizer:
        case EffectNodeKind::Dynamics:
        case EffectNodeKind::Space:
        case EffectNodeKind::Master:
        case EffectNodeKind::DeHum:
        case EffectNodeKind::ChannelRepair:
        case EffectNodeKind::SceneVfx:
        case EffectNodeKind::DelayVfx:
        case EffectNodeKind::ModulationVfx:
        case EffectNodeKind::DigitalDegradeVfx:
        case EffectNodeKind::RotaryVfx:
            return 0;
        }
        return 0;
    }

    void validate_buffer(
        const float* samples,
        std::size_t frame_count,
        std::size_t channel_count
    ) const {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("effect chain channel layout changed");
        }
    }

    void process_raw(float* samples, std::uint64_t* source_frames, std::size_t frame_count) {
        if (has_local_masks_ && frame_count > kMaximumProcessingFrames) {
            throw std::invalid_argument("effect chain block exceeds prepared scratch capacity");
        }
        for (std::size_t node_index = 0; node_index + 1 < node_count_; ++node_index) {
            const EffectNodeKind node = nodes_[node_index];
            const bool local = mask_plan_ != nullptr && mask_plan_->is_locally_masked(node);
            if (local) {
                std::copy_n(samples, frame_count * channel_count_, dry_samples_.data());
            }
            switch (node) {
            case EffectNodeKind::Restoration:
                if (restoration_enabled_) {
                    de_plosive_.process_interleaved(samples, frame_count, channel_count_);
                    noise_reducer_.process_interleaved(samples, frame_count, channel_count_);
                    de_esser_.process_interleaved(samples, frame_count, channel_count_);
                }
                break;
            case EffectNodeKind::DeHum:
                de_hum_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::DeClick:
                de_click_.process_interleaved(samples, frame_count, channel_count_);
                delay_source_anchors(node, source_frames, frame_count);
                break;
            case EffectNodeKind::ChannelRepair:
                channel_repair_.process_interleaved(samples, frame_count, channel_count_);
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
            case EffectNodeKind::SceneVfx:
                scene_vfx_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::DelayVfx:
                delay_vfx_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::ModulationVfx:
                modulation_vfx_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::TransformVfx:
                transform_vfx_.process_interleaved(samples, frame_count, channel_count_);
                delay_source_anchors(node, source_frames, frame_count);
                break;
            case EffectNodeKind::DigitalDegradeVfx:
                digital_degrade_vfx_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::DriveVfx:
                drive_vfx_.process_interleaved(samples, frame_count, channel_count_);
                delay_source_anchors(node, source_frames, frame_count);
                break;
            case EffectNodeKind::RotaryVfx:
                rotary_vfx_.process_interleaved(samples, frame_count, channel_count_);
                break;
            case EffectNodeKind::Master:
                break;
            }
            if (local) {
                for (std::size_t frame = 0; frame < frame_count; ++frame) {
                    const float mix = mask_plan_->mix_at(
                        node,
                        source_frames == nullptr ? kNoSourceFrame : source_frames[frame]
                    );
                    for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                        const std::size_t index = frame * channel_count_ + channel;
                        const float dry = dry_samples_[index];
                        samples[index] = dry + mix * (samples[index] - dry);
                    }
                }
            }
        }
    }

    void delay_source_anchors(
        EffectNodeKind node,
        std::uint64_t* source_frames,
        std::size_t frame_count
    ) {
        auto& delay = source_delays_[static_cast<std::size_t>(node)];
        auto& cursor = source_delay_cursors_[static_cast<std::size_t>(node)];
        if (source_frames == nullptr || delay.empty()) {
            return;
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::uint64_t delayed = delay[cursor];
            delay[cursor] = source_frames[frame];
            source_frames[frame] = delayed;
            cursor = (cursor + 1) % delay.size();
        }
    }

    std::size_t
    compact_output(float* samples, std::uint64_t* source_frames, std::size_t processed_frames) {
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
            if (source_frames != nullptr) {
                std::memmove(
                    source_frames,
                    source_frames + discarded,
                    produced * sizeof(std::uint64_t)
                );
            }
        }
        pending_output_frames_ -= produced;
        return produced;
    }

    void reset_compensation() {
        front_discard_frames_ = latency_frames_;
        pending_output_frames_ = 0;
        drain_input_frames_ = 0;
        finishing_ = false;
        for (auto& delay : source_delays_) {
            std::fill(delay.begin(), delay.end(), kNoSourceFrame);
        }
        source_delay_cursors_.fill(0);
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::array<EffectNodeKind, kEffectNodeCount> nodes_{};
    std::size_t node_count_ = 0;
    DePlosiveProcessor de_plosive_;
    AdaptiveNoiseReducer noise_reducer_;
    DeEsser de_esser_;
    DeHumFilter de_hum_;
    DeClickProcessor de_click_;
    ChannelRepairProcessor channel_repair_;
    ParametricEqualizer equalizer_;
    DynamicsProcessor dynamics_;
    AlgorithmicReverb reverb_;
    SceneVfxProcessor scene_vfx_;
    DelayVfxProcessor delay_vfx_;
    ModulationVfxProcessor modulation_vfx_;
    TransformVfxProcessor transform_vfx_;
    DigitalDegradeVfxProcessor digital_degrade_vfx_;
    DriveVfxProcessor drive_vfx_;
    RotaryVfxProcessor rotary_vfx_;
    bool restoration_enabled_ = true;
    const EffectMaskPlan* mask_plan_ = nullptr;
    bool has_local_masks_ = false;
    std::vector<float> dry_samples_;
    std::array<std::vector<std::uint64_t>, kEffectNodeCount> source_delays_;
    std::array<std::size_t, kEffectNodeCount> source_delay_cursors_{};
    std::size_t latency_frames_ = 0;
    std::size_t front_discard_frames_ = 0;
    std::size_t pending_output_frames_ = 0;
    std::size_t drain_input_frames_ = 0;
    bool finishing_ = false;
};

EffectProcessingChain::EffectProcessingChain(
    const PreparedAdjustment& adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count,
    const EffectMaskPlan* mask_plan
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count, mask_plan)) {}

EffectProcessingChain::~EffectProcessingChain() = default;

std::size_t EffectProcessingChain::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    return impl_->process_interleaved(samples, frame_count, channel_count);
}

std::size_t EffectProcessingChain::process_interleaved(
    float* samples,
    std::uint64_t* source_frames,
    std::size_t frame_count,
    std::size_t channel_count
) {
    return impl_->process_interleaved(samples, source_frames, frame_count, channel_count);
}

std::size_t EffectProcessingChain::finish_interleaved(
    float* samples,
    std::size_t capacity_frames,
    std::size_t channel_count
) {
    return impl_->finish_interleaved(samples, capacity_frames, channel_count);
}

std::size_t EffectProcessingChain::finish_interleaved(
    float* samples,
    std::uint64_t* source_frames,
    std::size_t capacity_frames,
    std::size_t channel_count
) {
    return impl_->finish_interleaved(samples, source_frames, capacity_frames, channel_count);
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

void EffectProcessingChain::update_channel_repair(ChannelRepairAdjustment adjustment) {
    impl_->update_channel_repair(adjustment);
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

void EffectProcessingChain::update_scene_vfx(SceneVfxAdjustment adjustment) {
    impl_->update_scene_vfx(adjustment);
}

void EffectProcessingChain::update_delay_vfx(DelayVfxAdjustment adjustment) {
    impl_->update_delay_vfx(adjustment);
}

void EffectProcessingChain::update_modulation_vfx(ModulationVfxAdjustment adjustment) {
    impl_->update_modulation_vfx(adjustment);
}

void EffectProcessingChain::update_transform_vfx(TransformVfxAdjustment adjustment) {
    impl_->update_transform_vfx(adjustment);
}

void EffectProcessingChain::update_digital_degrade_vfx(DigitalDegradeVfxAdjustment adjustment) {
    impl_->update_digital_degrade_vfx(adjustment);
}

void EffectProcessingChain::update_drive_vfx(DriveVfxAdjustment adjustment) {
    impl_->update_drive_vfx(adjustment);
}

void EffectProcessingChain::update_rotary_vfx(RotaryVfxAdjustment adjustment) {
    impl_->update_rotary_vfx(adjustment);
}

void EffectProcessingChain::validate_restoration(
    RestorationAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const DePlosiveProcessor de_plosive(
        adjustment.de_plosive,
        sample_rate,
        channel_count
    );
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

void EffectProcessingChain::validate_channel_repair(
    ChannelRepairAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const ChannelRepairProcessor processor(adjustment, sample_rate, channel_count);
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

void EffectProcessingChain::validate_scene_vfx(
    SceneVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    SceneVfxProcessor::validate(adjustment, sample_rate, channel_count);
}

void EffectProcessingChain::validate_delay_vfx(
    DelayVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const DelayVfxProcessor processor(adjustment, sample_rate, channel_count);
}

void EffectProcessingChain::validate_modulation_vfx(
    ModulationVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const ModulationVfxProcessor processor(adjustment, sample_rate, channel_count);
}

void EffectProcessingChain::validate_transform_vfx(
    TransformVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const TransformVfxProcessor processor(adjustment, sample_rate, channel_count);
}

void EffectProcessingChain::validate_digital_degrade_vfx(
    DigitalDegradeVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const DigitalDegradeVfxProcessor processor(
        adjustment,
        sample_rate,
        channel_count
    );
}

void EffectProcessingChain::validate_drive_vfx(
    DriveVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const DriveVfxProcessor processor(adjustment, sample_rate, channel_count);
}

void EffectProcessingChain::validate_rotary_vfx(
    RotaryVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    [[maybe_unused]] const RotaryVfxProcessor processor(adjustment, sample_rate, channel_count);
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
