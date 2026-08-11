#pragma once

#include "echo/audio/adjustment.hpp"
#include "echo/audio/effect_mask_plan.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Prepared, ordered insert processing for one immutable playback topology.
///
/// This owner keeps processor lifetime, reset/update policy, chain dispatch,
/// and fixed-latency compensation together. Input and returned output share
/// the caller's buffer; leading latency is removed in place and `finish()`
/// supplies the exact zero tail needed to preserve the authored frame count.
/// Low-cut, clip gain, fades, master limiting, metering, and device protection
/// remain explicit stages outside this movable insert chain.
class EffectProcessingChain {
  public:
    EffectProcessingChain(
        const PreparedAdjustment& adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count,
        const EffectMaskPlan* mask_plan = nullptr
    );
    ~EffectProcessingChain();

    EffectProcessingChain(const EffectProcessingChain&) = delete;
    EffectProcessingChain& operator=(const EffectProcessingChain&) = delete;

    /// Processes authored input frames and compacts latency-compensated output
    /// to the start of `samples`, returning the number of output frames.
    std::size_t
    process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    /// Source-aware form used by the prepared edit pipeline. `source_frames`
    /// is compacted and latency-aligned with the returned samples in place.
    std::size_t process_interleaved(
        float* samples,
        std::uint64_t* source_frames,
        std::size_t frame_count,
        std::size_t channel_count
    );

    /// After all authored input has been submitted, advances the chain with a
    /// bounded zero tail. Repeated calls return every delayed authored frame,
    /// then zero. No output beyond the authored input duration is produced.
    std::size_t
    finish_interleaved(float* samples, std::size_t capacity_frames, std::size_t channel_count);
    std::size_t finish_interleaved(
        float* samples,
        std::uint64_t* source_frames,
        std::size_t capacity_frames,
        std::size_t channel_count
    );

    void reset();

    void update_restoration(RestorationAdjustment adjustment);
    void update_de_hum(DeHumAdjustment adjustment);
    void update_de_click(DeClickAdjustment adjustment);
    void update_channel_repair(ChannelRepairAdjustment adjustment);
    void update_equalizer(ParametricEqualizerAdjustment adjustment);
    void update_compressor(CompressorAdjustment adjustment);
    void update_reverb(ReverbAdjustment adjustment);
    void update_scene_vfx(SceneVfxAdjustment adjustment);
    void update_delay_vfx(DelayVfxAdjustment adjustment);
    void update_modulation_vfx(ModulationVfxAdjustment adjustment);
    void update_transform_vfx(TransformVfxAdjustment adjustment);

    static void validate_restoration(
        RestorationAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_de_hum(
        DeHumAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_de_click(
        DeClickAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_channel_repair(
        ChannelRepairAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_equalizer(
        ParametricEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_compressor(CompressorAdjustment adjustment, std::uint32_t sample_rate);
    static void validate_reverb(
        ReverbAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_scene_vfx(
        SceneVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_delay_vfx(
        DelayVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_modulation_vfx(
        ModulationVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate_transform_vfx(
        TransformVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    [[nodiscard]] std::size_t latency_frames() const;
    [[nodiscard]] std::size_t pending_output_frames() const;
    [[nodiscard]] float gain_reduction_decibels() const;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
