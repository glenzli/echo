#include "echo/audio/output_limiter.hpp"

#include <algorithm>
#include <cmath>
#include <limits>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::size_t kLookAheadFrames = 64;

float amplitude_from_centibels(std::int16_t centibels) {
    return std::pow(10.0F, static_cast<float>(centibels) / 2000.0F);
}

float cubic(float p0, float p1, float p2, float p3, float t) {
    const float a = -0.5F * p0 + 1.5F * p1 - 1.5F * p2 + 0.5F * p3;
    const float b = p0 - 2.5F * p1 + 2.0F * p2 - 0.5F * p3;
    const float c = -0.5F * p0 + 0.5F * p2;
    return ((a * t + b) * t + c) * t + p1;
}

} // namespace

OutputLimiter::OutputLimiter(LimiterAdjustment adjustment, std::uint32_t sample_rate) :
    sample_rate_(sample_rate) {
    if (sample_rate_ == 0) {
        throw std::invalid_argument("limiter sample rate must be positive");
    }
    update(adjustment);
}

void OutputLimiter::update(LimiterAdjustment adjustment) {
    if (adjustment.ceiling_centibels < -600 || adjustment.ceiling_centibels > 0
        || adjustment.release_millis < 20 || adjustment.release_millis > 1000) {
        throw std::invalid_argument("limiter parameters are outside the supported range");
    }
    enabled_ = adjustment.enabled;
    ceiling_amplitude_ = amplitude_from_centibels(adjustment.ceiling_centibels);
    const float release_seconds = static_cast<float>(adjustment.release_millis) / 1000.0F;
    release_coefficient_ =
        1.0F - std::exp(-1.0F / (release_seconds * static_cast<float>(sample_rate_)));
}

void OutputLimiter::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    if (samples == nullptr || frame_count == 0 || channel_count == 0) {
        return;
    }
    for (std::size_t first = 0; first < frame_count; first += kLookAheadFrames) {
        const std::size_t end = std::min(first + kLookAheadFrames, frame_count);
        const float peak =
            enabled_ ? estimate_true_peak(samples, first, end, frame_count, channel_count) : 0.0F;
        const float target =
            enabled_ && peak > ceiling_amplitude_ ? ceiling_amplitude_ / peak : 1.0F;
        const float clamp_amplitude =
            enabled_ ? ceiling_amplitude_ : std::numeric_limits<float>::max();
        for (std::size_t frame = first; frame < end; ++frame) {
            if (target < gain_) {
                gain_ = target;
            } else {
                gain_ += (target - gain_) * release_coefficient_;
            }
            for (std::size_t channel = 0; channel < channel_count; ++channel) {
                const std::size_t index = frame * channel_count + channel;
                const float sample = std::isfinite(samples[index]) ? samples[index] : 0.0F;
                samples[index] = std::clamp(sample * gain_, -clamp_amplitude, clamp_amplitude);
            }
        }
    }
}

void OutputLimiter::reset() {
    gain_ = 1.0F;
}

float OutputLimiter::gain_reduction_decibels() const {
    return gain_ < 1.0F ? -20.0F * std::log10(std::max(gain_, 1.0E-9F)) : 0.0F;
}

float OutputLimiter::estimate_true_peak(
    const float* samples,
    std::size_t first_frame,
    std::size_t end_frame,
    std::size_t total_frames,
    std::size_t channel_count
) {
    float peak = 0.0F;
    const auto at = [&](std::size_t frame, std::size_t channel) {
        const std::size_t clamped = std::min(frame, total_frames - 1);
        const float sample = samples[clamped * channel_count + channel];
        return std::isfinite(sample) ? sample : 0.0F;
    };
    for (std::size_t frame = first_frame; frame < end_frame; ++frame) {
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            const float p0 = at(frame == 0 ? 0 : frame - 1, channel);
            const float p1 = at(frame, channel);
            const float p2 = at(frame + 1, channel);
            const float p3 = at(frame + 2, channel);
            peak = std::max(peak, std::abs(p1));
            peak = std::max(peak, std::abs(cubic(p0, p1, p2, p3, 0.25F)));
            peak = std::max(peak, std::abs(cubic(p0, p1, p2, p3, 0.50F)));
            peak = std::max(peak, std::abs(cubic(p0, p1, p2, p3, 0.75F)));
        }
    }
    return peak;
}

} // namespace echo::audio
