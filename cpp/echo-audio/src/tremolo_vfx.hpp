#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio::detail {

class TremoloVfx {
  public:
    TremoloVfx(TremoloAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);
    ~TremoloVfx();

    TremoloVfx(const TremoloVfx&) = delete;
    TremoloVfx& operator=(const TremoloVfx&) = delete;

    void update(TremoloAdjustment adjustment);
    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input);
    void reset();

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio::detail
