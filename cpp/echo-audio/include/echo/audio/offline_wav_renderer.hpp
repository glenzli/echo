#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <span>
#include <stdexcept>
#include <string>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Seekable byte destination supplied by the host so the engine remains
/// independent of Qt and platform file APIs.
class RenderByteSink {
  public:
    virtual ~RenderByteSink() = default;
    virtual void write(std::span<const std::byte> bytes) = 0;
    virtual void seek(std::uint64_t offset) = 0;
};

/// Cooperative callbacks for one bounded-memory offline render.
struct OfflineRenderCallbacks {
    std::function<bool()> cancelled;
    std::function<void(double)> progress;
};

/// Terminal measurements and format evidence for one rendered WAV.
struct OfflineWavRenderResult {
    std::uint64_t frame_count = 0;
    std::uint64_t size_bytes = 0;
    std::uint32_t sample_rate = 0;
    std::uint32_t channel_count = 0;
    std::uint16_t bit_depth = 0;
    float integrated_lufs = -70.0F;
    float true_peak_dbtp = -70.0F;
};

class OfflineRenderCancelled final : public std::runtime_error {
  public:
    OfflineRenderCancelled();
};

/// Streams the authored adjustment graph into a canonical 48 kHz stereo,
/// 24-bit PCM WAV. Memory use is constant in source duration.
class OfflineWavRenderer {
  public:
    [[nodiscard]] static OfflineWavRenderResult render(
        const std::string& sourcePath,
        const PlaybackAdjustment& adjustment,
        RenderByteSink& sink,
        const OfflineRenderCallbacks& callbacks = {}
    );
};

} // namespace echo::audio
