#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Deterministic controls for independent pitch, harmony, and formant colour.
///
/// Formant colour shifts three resonant bands; it is deliberately not presented
/// as AI voice conversion or identity preservation.
struct PitchVfxParameters {
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::int8_t pitch_semitones = 0;
    bool harmony_enabled = false;
    std::int8_t harmony_semitones = 7;
    std::uint8_t harmony_mix_percent = 35;
    std::int8_t formant_colour_semitones = 0;

    bool operator==(const PitchVfxParameters&) const = default;
};

/// Two-read-head, source-driven pitch shifting with optional harmony.
///
/// The processor has a fixed 12 ms causal delay, retained while bypassed so a
/// later effect-chain integration can compensate the timeline consistently.
class PitchVfxProcessor {
  public:
    PitchVfxProcessor(
        PitchVfxParameters parameters,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~PitchVfxProcessor();

    PitchVfxProcessor(const PitchVfxProcessor&) = delete;
    PitchVfxProcessor& operator=(const PitchVfxProcessor&) = delete;

    void update(PitchVfxParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] PitchVfxParameters parameters() const noexcept;
    [[nodiscard]] std::size_t latency_frames() const noexcept;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
