#include "echo/audio/loudness_meter.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr double kLoudnessOffset = -0.691;
constexpr float kMeterFloorDecibels = -70.0F;

} // namespace

LoudnessMeter::LoudnessMeter(std::uint32_t sample_rate, std::size_t channel_count) :
    sample_rate_(sample_rate), channel_count_(channel_count),
    weighting_(sample_rate, channel_count),
    energy_window_(static_cast<std::size_t>(sample_rate) * 400U / 1000U, 0.0) {
    if (sample_rate_ == 0 || channel_count_ == 0 || energy_window_.empty()) {
        throw std::invalid_argument("loudness meter requires valid audio dimensions");
    }
    // Peak indication releases by 20 dB per second while still catching every
    // prepared sample. This is a display hold, not a limiter or authored DSP.
    peak_release_per_frame_ = std::pow(10.0, -20.0 / (20.0 * sample_rate_));
}

void LoudnessMeter::reset() {
    weighting_.reset();
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
        const double frame_energy =
            weighting_.process_frame(samples + frame * channel_count_, channel_count_);
        double frame_peak = 0.0;
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            const double sample = samples[frame * channel_count_ + channel];
            frame_peak = std::max(frame_peak, std::abs(sample));
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

} // namespace echo::audio
