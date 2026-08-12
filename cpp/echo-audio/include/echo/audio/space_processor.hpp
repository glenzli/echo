#pragma once

#include "echo/audio/adjustment.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

namespace echo::audio {

/// Executes the mutually exclusive algorithmic/convolution Space insert.
///
/// Bank construction and replacement happen on the playback producer thread.
/// Processing is allocation-free: an identity change fades the old wet signal
/// to dry, resets history, then fades the replacement wet signal in.
class SpaceProcessor {
  public:
    SpaceProcessor(
        const SpaceAdjustment& adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    ~SpaceProcessor();

    SpaceProcessor(const SpaceProcessor&) = delete;
    SpaceProcessor& operator=(const SpaceProcessor&) = delete;

    void update(const SpaceAdjustment& adjustment);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
