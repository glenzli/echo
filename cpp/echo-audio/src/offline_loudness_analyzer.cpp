#include "echo/audio/offline_loudness_analyzer.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr double kLoudnessOffset = -0.691;
constexpr double kAbsoluteGateLufs = -70.0;
constexpr float kResultFloor = -70.0F;

double loudness_from_energy(double energy) {
    return energy > 0.0 ? kLoudnessOffset + 10.0 * std::log10(energy) : -INFINITY;
}

float cubic(float p0, float p1, float p2, float p3, float t) {
    const float a = -0.5F * p0 + 1.5F * p1 - 1.5F * p2 + 0.5F * p3;
    const float b = p0 - 2.5F * p1 + 2.0F * p2 - 0.5F * p3;
    const float c = -0.5F * p0 + 0.5F * p2;
    return ((a * t + b) * t + c) * t + p1;
}

} // namespace

OfflineLoudnessAnalyzer::OfflineLoudnessAnalyzer(
    std::uint32_t sample_rate,
    std::size_t channel_count
) :
    sample_rate_(sample_rate), channel_count_(channel_count),
    weighting_(sample_rate, channel_count),
    energy_window_(static_cast<std::size_t>(sample_rate) * 400U / 1000U, 0.0),
    block_step_frames_(static_cast<std::size_t>(sample_rate) * 100U / 1000U),
    true_peak_history_(channel_count) {
    if (sample_rate_ == 0 || channel_count_ == 0 || energy_window_.empty()
        || block_step_frames_ == 0) {
        throw std::invalid_argument("offline loudness analysis requires valid audio dimensions");
    }
}

void OfflineLoudnessAnalyzer::process_interleaved(
    const float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || channel_count != channel_count_) {
        throw std::invalid_argument("offline loudness analysis channel layout changed");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float* const input = samples + frame * channel_count_;
        const double energy = weighting_.process_frame(input, channel_count_);
        total_energy_ += energy;
        ++total_frames_;
        energy_sum_ -= energy_window_[energy_cursor_];
        energy_window_[energy_cursor_] = energy;
        energy_sum_ += energy;
        energy_cursor_ = (energy_cursor_ + 1) % energy_window_.size();
        energy_count_ = std::min(energy_count_ + 1, energy_window_.size());
        ++frames_since_block_;
        if (energy_count_ == energy_window_.size() && frames_since_block_ >= block_step_frames_) {
            block_energies_.push_back(energy_sum_ / static_cast<double>(energy_window_.size()));
            frames_since_block_ = 0;
        }

        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            auto& history = true_peak_history_[channel];
            history[0] = history[1];
            history[1] = history[2];
            history[2] = history[3];
            history[3] = std::isfinite(input[channel]) ? input[channel] : 0.0F;
            true_peak_amplitude_ =
                std::max(true_peak_amplitude_, static_cast<double>(std::abs(history[3])));
            if (true_peak_history_count_ >= kTruePeakHistory - 1) {
                for (const float phase : {0.25F, 0.50F, 0.75F}) {
                    true_peak_amplitude_ = std::max(
                        true_peak_amplitude_,
                        static_cast<double>(
                            std::abs(cubic(history[0], history[1], history[2], history[3], phase))
                        )
                    );
                }
            }
        }
        true_peak_history_count_ = std::min(true_peak_history_count_ + 1, kTruePeakHistory);
    }
}

OfflineLoudnessResult OfflineLoudnessAnalyzer::result() const {
    std::vector<double> blocks = block_energies_;
    if (blocks.empty() && total_frames_ > 0) {
        blocks.push_back(total_energy_ / static_cast<double>(total_frames_));
    }
    double absolute_sum = 0.0;
    std::size_t absolute_count = 0;
    for (const double energy : blocks) {
        if (loudness_from_energy(energy) >= kAbsoluteGateLufs) {
            absolute_sum += energy;
            ++absolute_count;
        }
    }
    float integrated = kResultFloor;
    if (absolute_count > 0) {
        const double absolute_mean = absolute_sum / static_cast<double>(absolute_count);
        const double relative_gate = loudness_from_energy(absolute_mean) - 10.0;
        double relative_sum = 0.0;
        std::size_t relative_count = 0;
        for (const double energy : blocks) {
            const double loudness = loudness_from_energy(energy);
            if (loudness >= kAbsoluteGateLufs && loudness >= relative_gate) {
                relative_sum += energy;
                ++relative_count;
            }
        }
        if (relative_count > 0) {
            integrated = static_cast<float>(
                loudness_from_energy(relative_sum / static_cast<double>(relative_count))
            );
        }
    }
    const float true_peak = true_peak_amplitude_ > 0.0
                                ? static_cast<float>(20.0 * std::log10(true_peak_amplitude_))
                                : kResultFloor;
    return {
        .integrated_lufs = std::clamp(integrated, kResultFloor, 12.0F),
        .true_peak_dbtp = std::clamp(true_peak, kResultFloor, 12.0F),
    };
}

} // namespace echo::audio
