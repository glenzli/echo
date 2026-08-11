#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio::detail {

class PhaserVfx {
  public:
    PhaserVfx(PhaserAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);
    ~PhaserVfx();

    PhaserVfx(const PhaserVfx&) = delete;
    PhaserVfx& operator=(const PhaserVfx&) = delete;

    void update(PhaserAdjustment adjustment);
    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input);
    void reset();

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio::detail
