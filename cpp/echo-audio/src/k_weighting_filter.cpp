#include "echo/audio/k_weighting_filter.hpp"

#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr double kPi = 3.14159265358979323846;

} // namespace

KWeightingFilter::KWeightingFilter(std::uint32_t sample_rate, std::size_t channel_count) :
    channel_count_(channel_count), channels_(channel_count), shelf_(high_shelf(sample_rate)),
    high_pass_(high_pass(sample_rate)) {
    if (sample_rate == 0 || channel_count_ == 0) {
        throw std::invalid_argument("K-weighting requires valid audio dimensions");
    }
}

void KWeightingFilter::reset() {
    channels_.assign(channel_count_, {});
}

double KWeightingFilter::process_frame(const float* interleaved_frame, std::size_t channel_count) {
    if (interleaved_frame == nullptr || channel_count != channel_count_) {
        throw std::invalid_argument("K-weighting channel layout changed");
    }
    double energy = 0.0;
    for (std::size_t channel = 0; channel < channel_count_; ++channel) {
        const double sample = std::isfinite(interleaved_frame[channel])
                                  ? static_cast<double>(interleaved_frame[channel])
                                  : 0.0;
        double weighted = process_biquad(sample, channels_[channel].shelf, shelf_);
        weighted = process_biquad(weighted, channels_[channel].high_pass, high_pass_);
        energy += weighted * weighted;
    }
    return energy;
}

KWeightingFilter::BiquadCoefficients KWeightingFilter::high_shelf(std::uint32_t sample_rate) {
    if (sample_rate == 0) {
        throw std::invalid_argument("K-weighting sample rate must be positive");
    }
    constexpr double frequency = 1681.974450955533;
    constexpr double gain_decibels = 3.999843853973347;
    constexpr double quality = 0.7071752369554196;
    constexpr double exponent = 0.4996667741545416;
    const double k = std::tan(kPi * frequency / sample_rate);
    const double vh = std::pow(10.0, gain_decibels / 20.0);
    const double vb = std::pow(vh, exponent);
    const double denominator = 1.0 + k / quality + k * k;
    return {
        .b0 = (vh + vb * k / quality + k * k) / denominator,
        .b1 = 2.0 * (k * k - vh) / denominator,
        .b2 = (vh - vb * k / quality + k * k) / denominator,
        .a1 = 2.0 * (k * k - 1.0) / denominator,
        .a2 = (1.0 - k / quality + k * k) / denominator,
    };
}

KWeightingFilter::BiquadCoefficients KWeightingFilter::high_pass(std::uint32_t sample_rate) {
    if (sample_rate == 0) {
        throw std::invalid_argument("K-weighting sample rate must be positive");
    }
    constexpr double frequency = 38.13547087602444;
    constexpr double quality = 0.5003270373238773;
    const double k = std::tan(kPi * frequency / sample_rate);
    const double denominator = 1.0 + k / quality + k * k;
    return {
        .b0 = 1.0 / denominator,
        .b1 = -2.0 / denominator,
        .b2 = 1.0 / denominator,
        .a1 = 2.0 * (k * k - 1.0) / denominator,
        .a2 = (1.0 - k / quality + k * k) / denominator,
    };
}

double KWeightingFilter::process_biquad(
    double sample,
    BiquadState& state,
    const BiquadCoefficients& coefficients
) {
    const double output = coefficients.b0 * sample + coefficients.b1 * state.x1
                          + coefficients.b2 * state.x2 - coefficients.a1 * state.y1
                          - coefficients.a2 * state.y2;
    state.x2 = state.x1;
    state.x1 = sample;
    state.y2 = state.y1;
    state.y1 = output;
    return output;
}

} // namespace echo::audio
