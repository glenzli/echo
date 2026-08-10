#include "echo/audio/loudness_meter.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr double kPi = 3.14159265358979323846;
constexpr double kLoudnessOffset = -0.691;
constexpr float kMeterFloorDecibels = -70.0F;

} // namespace

LoudnessMeter::LoudnessMeter(std::uint32_t sample_rate, std::size_t channel_count) :
    sample_rate_(sample_rate), channel_count_(channel_count), channels_(channel_count),
    shelf_(high_shelf(sample_rate)), high_pass_(high_pass(sample_rate)),
    energy_window_(static_cast<std::size_t>(sample_rate) * 400U / 1000U, 0.0) {
    if (sample_rate_ == 0 || channel_count_ == 0 || energy_window_.empty()) {
        throw std::invalid_argument("loudness meter requires valid audio dimensions");
    }
    // Peak indication releases by 20 dB per second while still catching every
    // prepared sample. This is a display hold, not a limiter or authored DSP.
    peak_release_per_frame_ = std::pow(10.0, -20.0 / (20.0 * sample_rate_));
}

void LoudnessMeter::reset() {
    channels_.assign(channel_count_, {});
    std::fill(energy_window_.begin(), energy_window_.end(), 0.0);
    energy_cursor_ = 0;
    energy_count_ = 0;
    energy_sum_ = 0.0;
    peak_amplitude_ = 0.0;
}

void LoudnessMeter::process_interleaved(
    const float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || channel_count != channel_count_) {
        throw std::invalid_argument("loudness meter channel layout changed");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        double frame_energy = 0.0;
        double frame_peak = 0.0;
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const double sample = samples[frame * channel_count_ + channel];
            frame_peak = std::max(frame_peak, std::abs(sample));
            double weighted = process_biquad(sample, channels_[channel].shelf, shelf_);
            weighted = process_biquad(weighted, channels_[channel].high_pass, high_pass_);
            frame_energy += weighted * weighted;
        }
        energy_sum_ -= energy_window_[energy_cursor_];
        energy_window_[energy_cursor_] = frame_energy;
        energy_sum_ += frame_energy;
        energy_cursor_ = (energy_cursor_ + 1) % energy_window_.size();
        energy_count_ = std::min(energy_count_ + 1, energy_window_.size());
        peak_amplitude_ = std::max(frame_peak, peak_amplitude_ * peak_release_per_frame_);
    }
}

LoudnessSnapshot LoudnessMeter::snapshot() const {
    const double mean_square =
        energy_count_ > 0 ? energy_sum_ / static_cast<double>(energy_count_) : 0.0;
    const float momentary =
        mean_square > 0.0 ? static_cast<float>(kLoudnessOffset + 10.0 * std::log10(mean_square))
                          : kMeterFloorDecibels;
    const float peak = peak_amplitude_ > 0.0
                           ? static_cast<float>(20.0 * std::log10(peak_amplitude_))
                           : kMeterFloorDecibels;
    return {
        .momentary_lufs = std::clamp(momentary, kMeterFloorDecibels, 12.0F),
        .sample_peak_dbfs = std::clamp(peak, kMeterFloorDecibels, 12.0F),
    };
}

LoudnessMeter::BiquadCoefficients LoudnessMeter::high_shelf(std::uint32_t sample_rate) {
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

LoudnessMeter::BiquadCoefficients LoudnessMeter::high_pass(std::uint32_t sample_rate) {
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

double LoudnessMeter::process_biquad(
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
