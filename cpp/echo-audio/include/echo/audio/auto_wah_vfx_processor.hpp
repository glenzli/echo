#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Input-envelope controls for a linked-channel Auto-Wah filter.
struct AutoWahVfxParameters {
    bool enabled = false;
    std::uint8_t mix_percent = 70;
    std::uint8_t sensitivity_percent = 55;
    std::uint16_t minimum_frequency_hertz = 280;
    std::uint16_t maximum_frequency_hertz = 2800;
    std::uint8_t resonance_tenths = 18;

    bool operator==(const AutoWahVfxParameters&) const = default;
};

/// A zero-latency, input-driven resonant filter with a linked stereo envelope.
class AutoWahVfxProcessor {
  public:
    AutoWahVfxProcessor(
        AutoWahVfxParameters parameters,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~AutoWahVfxProcessor();

    AutoWahVfxProcessor(const AutoWahVfxProcessor&) = delete;
    AutoWahVfxProcessor& operator=(const AutoWahVfxProcessor&) = delete;

    void update(AutoWahVfxParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] AutoWahVfxParameters parameters() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
