#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Deterministic, deliberately stylized voice transformations for Creative VFX.
///
/// These effects do not infer, clone, or synthesize a speaker identity. Robot
/// and Ghost use fixed ring/frequency modulation; Monster, Tiny, and Giant use
/// a bounded dual-read-head time-domain pitch effect. The complete topology is
/// prepared during construction. `update()` and `process_interleaved()` do not
/// allocate, and every character keeps the same 50 ms processing latency while
/// present so bypass and character changes remain time-aligned.
class TransformVfxProcessor {
  public:
    TransformVfxProcessor(
        TransformVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    void update(TransformVfxAdjustment adjustment);
    void reset();
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);

    [[nodiscard]] std::size_t latency_frames() const noexcept;
    [[nodiscard]] TransformVfxAdjustment adjustment() const noexcept;

  private:
    static constexpr std::size_t kCharacterCount = 5;
    static constexpr std::size_t kHilbertRadius = 15;

    static void validate(
        TransformVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    [[nodiscard]] static std::size_t character_index(TransformVfxCharacter character);
    [[nodiscard]] float read_delay(std::size_t channel, float delay_frames) const;
    [[nodiscard]] float delayed_sample(std::size_t channel, std::size_t delay_frames) const;
    [[nodiscard]] float
    pitch_sample(TransformVfxCharacter character, std::size_t channel, float amount);
    [[nodiscard]] float robot_sample(std::size_t channel, float amount);
    [[nodiscard]] float ghost_sample(std::size_t channel, float amount);
    [[nodiscard]] float
    character_sample(TransformVfxCharacter character, std::size_t channel, float amount);
    void advance_character(TransformVfxCharacter character, float amount);

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t latency_frames_ = 0;
    std::size_t pitch_range_frames_ = 0;
    std::size_t buffer_frames_ = 0;
    std::size_t write_cursor_ = 0;
    std::vector<float> delay_buffer_;
    std::vector<float> giant_lowpass_state_;

    TransformVfxAdjustment target_;
    TransformVfxCharacter from_character_ = TransformVfxCharacter::Robot;
    TransformVfxCharacter to_character_ = TransformVfxCharacter::Robot;
    std::size_t character_transition_frames_ = 0;
    std::size_t character_transition_remaining_ = 0;

    float smoothing_coefficient_ = 0.0F;
    float current_mix_ = 0.0F;
    float target_mix_ = 0.0F;
    float current_amount_ = 0.5F;
    float target_amount_ = 0.5F;
    std::array<float, kCharacterCount> pitch_phase_{};
    std::array<float, kCharacterCount> oscillator_phase_{};
    std::array<float, kHilbertRadius * 2 + 1> hilbert_coefficients_{};
};

} // namespace echo::audio
