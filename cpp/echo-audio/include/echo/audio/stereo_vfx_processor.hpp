#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Input-driven mid/side width and equal-power pan parameters.
struct StereoVfxParameters {
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::uint8_t width_percent = 100;
    std::int8_t pan_percent = 0;

    bool operator==(const StereoVfxParameters&) const = default;
};

/// Zero-latency stereo utility. It never creates an independent ambience
/// signal and leaves mono sources structurally mono.
class StereoVfxProcessor {
  public:
    StereoVfxProcessor(
        StereoVfxParameters parameters,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~StereoVfxProcessor();

    StereoVfxProcessor(const StereoVfxProcessor&) = delete;
    StereoVfxProcessor& operator=(const StereoVfxProcessor&) = delete;

    void update(StereoVfxParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset() noexcept;

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] StereoVfxParameters parameters() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
