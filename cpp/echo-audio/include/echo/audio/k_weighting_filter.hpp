#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Stateful BS.1770 K-weighting filter bank for one fixed channel layout.
class KWeightingFilter {
  public:
    KWeightingFilter(std::uint32_t sample_rate, std::size_t channel_count);

    void reset();
    [[nodiscard]] double process_frame(const float* interleaved_frame, std::size_t channel_count);

  private:
    struct BiquadState {
        double x1 = 0.0;
        double x2 = 0.0;
        double y1 = 0.0;
        double y2 = 0.0;
    };

    struct ChannelState {
        BiquadState shelf;
        BiquadState high_pass;
    };

    struct BiquadCoefficients {
        double b0 = 1.0;
        double b1 = 0.0;
        double b2 = 0.0;
        double a1 = 0.0;
        double a2 = 0.0;
    };

    [[nodiscard]] static BiquadCoefficients high_shelf(std::uint32_t sample_rate);
    [[nodiscard]] static BiquadCoefficients high_pass(std::uint32_t sample_rate);
    [[nodiscard]] static double
    process_biquad(double sample, BiquadState& state, const BiquadCoefficients& coefficients);

    std::size_t channel_count_ = 0;
    std::vector<ChannelState> channels_;
    BiquadCoefficients shelf_;
    BiquadCoefficients high_pass_;
};

} // namespace echo::audio
