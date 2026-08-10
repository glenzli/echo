#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <span>
#include <stdexcept>

namespace echo::audio {

/// Seekable byte destination supplied by the host so offline encoders remain
/// independent of Qt and platform file APIs.
class RenderByteSink {
  public:
    virtual ~RenderByteSink() = default;
    virtual void write(std::span<const std::byte> bytes) = 0;
    virtual void seek(std::uint64_t offset) = 0;
};

struct OfflineRenderCallbacks {
    std::function<bool()> cancelled;
    std::function<void(double)> progress;
};

struct OfflineRenderResult {
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

} // namespace echo::audio
