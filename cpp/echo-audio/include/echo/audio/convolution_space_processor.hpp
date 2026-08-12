#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <span>

namespace echo::audio {

struct ConvolutionSpaceAdjustment {
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::int16_t wet_gain_centibels = 0;
};

enum class PreparedImpulseLayout : std::uint8_t {
    Mono = 0,
    StereoParallel = 1,
};

/// Realtime convolution over an already verified, canonical 48 kHz impulse.
///
/// A mono impulse drives each program channel independently. StereoParallel
/// means L→L and R→R; it is deliberately not described as true stereo, which
/// would require four LL/LR/RL/RR paths. IR decoding, resampling, durable source
/// identity, and hot bank replacement belong to preparation/lifecycle owners.
/// Construction performs all allocations and FFT preparation. `update()`,
/// `process_interleaved()`, and `reset()` allocate nothing and take no locks.
class ConvolutionSpaceProcessor {
  public:
    ConvolutionSpaceProcessor(
        ConvolutionSpaceAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count,
        std::span<const float> impulse_left,
        std::span<const float> impulse_right = {}
    );
    ~ConvolutionSpaceProcessor();

    ConvolutionSpaceProcessor(const ConvolutionSpaceProcessor&) = delete;
    ConvolutionSpaceProcessor& operator=(const ConvolutionSpaceProcessor&) = delete;

    void update(ConvolutionSpaceAdjustment adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] bool is_bypassed() const noexcept;
    [[nodiscard]] ConvolutionSpaceAdjustment adjustment() const noexcept;
    [[nodiscard]] PreparedImpulseLayout impulse_layout() const noexcept;
    [[nodiscard]] std::size_t impulse_frames() const noexcept;
    [[nodiscard]] std::size_t tail_frames() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 0;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
