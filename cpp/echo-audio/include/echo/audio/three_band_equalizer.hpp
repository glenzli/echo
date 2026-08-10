#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Prepared three-band restoration equalizer.
///
/// Coefficients and per-channel state are allocated before decoding starts;
/// the producer thread performs only bounded scalar DSP for each sample.
class ThreeBandEqualizer {
  public:
    ThreeBandEqualizer(
        ThreeBandEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    [[nodiscard]] float process_sample(float sample, std::size_t channel);
    void reset();
    [[nodiscard]] bool is_bypassed() const;

  private:
    struct State {
        float z1 = 0.0F;
        float z2 = 0.0F;
    };

    struct Section {
        float b0 = 1.0F;
        float b1 = 0.0F;
        float b2 = 0.0F;
        float a1 = 0.0F;
        float a2 = 0.0F;
        std::vector<State> states;

        [[nodiscard]] float process(float sample, std::size_t channel);
        void reset();
    };

    struct Coefficients {
        double b0;
        double b1;
        double b2;
        double a0;
        double a1;
        double a2;
    };

    static Section normalized(Coefficients coefficients, std::size_t channel_count);
    static Section
    low_shelf(std::int16_t gain_centibels, std::uint32_t sample_rate, std::size_t channel_count);
    static Section
    peaking(std::int16_t gain_centibels, std::uint32_t sample_rate, std::size_t channel_count);
    static Section
    high_shelf(std::int16_t gain_centibels, std::uint32_t sample_rate, std::size_t channel_count);

    Section low_;
    Section mid_;
    Section high_;
    bool bypassed_ = true;
};

} // namespace echo::audio
