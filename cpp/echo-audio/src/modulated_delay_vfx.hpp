#pragma once

#include "echo/audio/creative_vfx.hpp"

#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio::detail {

class ChorusVfx {
  public:
    ChorusVfx(ChorusAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);
    ~ChorusVfx();

    ChorusVfx(const ChorusVfx&) = delete;
    ChorusVfx& operator=(const ChorusVfx&) = delete;

    void update(ChorusAdjustment adjustment);
    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input);
    void reset();

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

class FlangerVfx {
  public:
    FlangerVfx(FlangerAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count);
    ~FlangerVfx();

    FlangerVfx(const FlangerVfx&) = delete;
    FlangerVfx& operator=(const FlangerVfx&) = delete;

    void update(FlangerAdjustment adjustment);
    [[nodiscard]] std::array<float, 2> process(const std::array<float, 2>& input);
    void reset();

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio::detail
