#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Deterministic causal spectral freeze for canonical mono/stereo PCM.
///
/// A 4096-frame, 75%-overlapped STFT captures only already-observed source
/// audio. The complete FFT, history, overlap-add, and phase workspace is
/// allocated during construction; `update()`, `request_capture()`,
/// `process_interleaved()`, and `reset()` allocate nothing and take no locks.
/// The node always retains its fixed 4096-frame latency while present, even
/// when disabled or at zero mix, so the shared effect chain can compensate it
/// without timeline jumps.
class FreezeVfxProcessor {
  public:
    FreezeVfxProcessor(
        FreezeVfxAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~FreezeVfxProcessor();

    FreezeVfxProcessor(const FreezeVfxProcessor&) = delete;
    FreezeVfxProcessor& operator=(const FreezeVfxProcessor&) = delete;

    void update(FreezeVfxAdjustment adjustment);

    /// Arms the first capture after reset.
    ///
    /// The request is rejected until one complete analysis window has already
    /// been processed; the host must pre-roll that source history before the
    /// authored anchor. An accepted request captures the next processed source
    /// frame exactly. Re-capture requires a separately prepared processor/bank
    /// so structural anchor changes can be crossfaded by their lifecycle owner
    /// instead of overwriting live history.
    [[nodiscard]] bool request_capture() noexcept;

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset() noexcept;

    [[nodiscard]] bool has_capture() const noexcept;
    [[nodiscard]] bool capture_pending() const noexcept;
    [[nodiscard]] FreezeVfxAdjustment adjustment() const noexcept;
    [[nodiscard]] static constexpr std::size_t latency_frames() noexcept {
        return 4096;
    }

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
