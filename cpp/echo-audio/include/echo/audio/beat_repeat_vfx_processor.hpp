#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// A bounded repeat slice, with an optional reverse read of the captured source.
struct BeatRepeatVfxParameters {
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::uint16_t slice_millis = 125;
    std::uint8_t repeat_count = 2;
    bool reverse = false;

    bool operator==(const BeatRepeatVfxParameters&) const = default;
};

/// Fixed-latency source-buffer beat repeat. The preallocated history makes
/// reverse repeat deterministic in preview and offline renders.
class BeatRepeatVfxProcessor {
  public:
    BeatRepeatVfxProcessor(
        BeatRepeatVfxParameters parameters,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~BeatRepeatVfxProcessor();

    BeatRepeatVfxProcessor(const BeatRepeatVfxProcessor&) = delete;
    BeatRepeatVfxProcessor& operator=(const BeatRepeatVfxProcessor&) = delete;

    void update(BeatRepeatVfxParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] BeatRepeatVfxParameters parameters() const noexcept;
    [[nodiscard]] std::size_t latency_frames() const noexcept;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
