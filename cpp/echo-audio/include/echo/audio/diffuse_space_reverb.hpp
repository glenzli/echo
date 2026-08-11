#pragma once

#include <array>
#include <cstdint>
#include <memory>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Echo-owned diffuse Hall/Plate processor prepared outside the audio callback.
///
/// The implementation is a clean-room eight-line feedback delay network. Its
/// storage and coefficients are fixed at construction, so `process_frame()`
/// performs no allocation and reports no infrastructure latency. Pre-delay and
/// the first wet reflection are authored acoustic content, not chain latency.
class DiffuseSpaceReverb {
  public:
    DiffuseSpaceReverb(ReverbAdjustment adjustment, std::uint32_t sample_rate);
    ~DiffuseSpaceReverb();

    DiffuseSpaceReverb(const DiffuseSpaceReverb&) = delete;
    DiffuseSpaceReverb& operator=(const DiffuseSpaceReverb&) = delete;

    [[nodiscard]] std::array<float, 2> process_frame(float left, float right);
    void reset();

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
