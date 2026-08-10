#include "echo/audio/output_guard.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr float kCeiling = 0.995F;
constexpr float kCeilingShoulder = 0.004F;
constexpr float kAttackSeconds = 0.001F;
constexpr float kReleaseSeconds = 0.080F;

float smoothing_coefficient(float seconds, std::uint32_t sample_rate) {
    return 1.0F - std::exp(-1.0F / (seconds * static_cast<float>(sample_rate)));
}

} // namespace

OutputGuard::OutputGuard(std::uint32_t sample_rate) {
    if (sample_rate == 0) {
        throw std::invalid_argument("output guard sample rate must be positive");
    }
    attack_coefficient_ = smoothing_coefficient(kAttackSeconds, sample_rate);
    release_coefficient_ = smoothing_coefficient(kReleaseSeconds, sample_rate);
}

float OutputGuard::soft_ceiling(float sample) {
    if (!std::isfinite(sample)) {
        return 0.0F;
    }
    const float magnitude = std::abs(sample);
    if (magnitude <= kCeiling) {
        return sample;
    }
    const float bounded =
        kCeiling + kCeilingShoulder * std::tanh((magnitude - kCeiling) / kCeilingShoulder);
    return std::copysign(bounded, sample);
}

void OutputGuard::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || frame_count == 0 || channel_count == 0) {
        return;
    }
    float peak = 0.0F;
    for (std::size_t index = 0; index < frame_count * channel_count; ++index) {
        if (std::isfinite(samples[index])) {
            peak = std::max(peak, std::abs(samples[index]));
        }
    }
    const float target_gain = peak > kCeiling ? kCeiling / peak : 1.0F;
    if (!initialized_) {
        gain_ = target_gain;
        initialized_ = true;
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        const float coefficient = target_gain < gain_ ? attack_coefficient_ : release_coefficient_;
        gain_ += (target_gain - gain_) * coefficient;
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            const std::size_t index = frame * channel_count + channel;
            samples[index] = soft_ceiling(samples[index] * gain_);
        }
    }
}

void OutputGuard::reset() {
    gain_ = 1.0F;
    initialized_ = false;
}

} // namespace echo::audio
