#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Bounded, input-driven controls for a stylized tape motion effect.
///
/// Dropout attenuates existing input; it never introduces hiss, crackle, or
/// another independent sound source. `update()` and processing allocate no
/// memory after construction.
struct TapeVfxParameters {
    bool enabled = false;
    std::uint8_t mix_percent = 55;
    std::uint8_t saturation_percent = 25;
    std::uint8_t wow_flutter_percent = 30;
    std::uint8_t dropout_percent = 0;

    bool operator==(const TapeVfxParameters&) const = default;
};

class TapeVfxProcessor {
  public:
    TapeVfxProcessor(
        TapeVfxParameters parameters,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~TapeVfxProcessor();

    TapeVfxProcessor(const TapeVfxProcessor&) = delete;
    TapeVfxProcessor& operator=(const TapeVfxProcessor&) = delete;

    void update(TapeVfxParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] TapeVfxParameters parameters() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
