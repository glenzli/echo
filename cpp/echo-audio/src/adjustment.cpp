#include "echo/audio/adjustment.hpp"

#include <algorithm>
#include <cmath>
#include <limits>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr std::int16_t kMinimumGainCentibels = -2400;
constexpr std::int16_t kMaximumGainCentibels = 1200;
constexpr float kHalfPi = 1.5707963267948966F;

bool valid_curve(FadeCurve curve) {
    switch (curve) {
    case FadeCurve::Linear:
    case FadeCurve::Smooth:
    case FadeCurve::EqualPower:
        return true;
    }
    return false;
}

float evaluate_curve(float progress, FadeCurve curve) {
    const float bounded = std::clamp(progress, 0.0F, 1.0F);
    switch (curve) {
    case FadeCurve::Linear:
        return bounded;
    case FadeCurve::Smooth:
        return bounded * bounded * (3.0F - 2.0F * bounded);
    case FadeCurve::EqualPower:
        return std::sin(bounded * kHalfPi);
    }
    return bounded;
}

std::uint64_t milliseconds_to_frames(std::uint64_t millis, std::uint32_t sample_rate) {
    if (sample_rate == 0) {
        throw std::invalid_argument("adjustment sample rate must be positive");
    }
    if (millis > std::numeric_limits<std::uint64_t>::max() / sample_rate) {
        throw std::invalid_argument("adjustment time exceeds the supported range");
    }
    return millis * sample_rate / 1000;
}

} // namespace

PreparedAdjustment::PreparedAdjustment(
    PlaybackAdjustment authored,
    std::uint64_t source_duration_millis,
    std::uint32_t sample_rate
) {
    if (source_duration_millis == 0) {
        throw std::invalid_argument("adjustment requires a known source duration");
    }
    if (authored.trim_end_millis == 0) {
        authored.trim_end_millis = source_duration_millis;
    }
    if (authored.trim_start_millis >= authored.trim_end_millis
        || authored.trim_end_millis > source_duration_millis) {
        throw std::invalid_argument("adjustment trim range is outside the source");
    }
    const std::uint64_t selected_millis = authored.trim_end_millis - authored.trim_start_millis;
    if (authored.fade_in_millis + authored.fade_out_millis > selected_millis) {
        throw std::invalid_argument("adjustment fades overlap");
    }
    if (authored.gain_centibels < kMinimumGainCentibels
        || authored.gain_centibels > kMaximumGainCentibels) {
        throw std::invalid_argument("adjustment gain is outside the supported range");
    }
    if (!valid_curve(authored.fade_in_curve) || !valid_curve(authored.fade_out_curve)) {
        throw std::invalid_argument("adjustment fade curve is outside the supported range");
    }

    trim_start_millis_ = authored.trim_start_millis;
    trim_end_millis_ = authored.trim_end_millis;
    start_frame_ = milliseconds_to_frames(trim_start_millis_, sample_rate);
    end_frame_ = milliseconds_to_frames(trim_end_millis_, sample_rate);
    fade_in_frames_ = milliseconds_to_frames(authored.fade_in_millis, sample_rate);
    fade_out_frames_ = milliseconds_to_frames(authored.fade_out_millis, sample_rate);
    fade_in_curve_ = authored.fade_in_curve;
    fade_out_curve_ = authored.fade_out_curve;
    gain_amplitude_ = std::pow(10.0F, static_cast<float>(authored.gain_centibels) / 2000.0F);
}

std::uint64_t PreparedAdjustment::start_frame() const {
    return start_frame_;
}

std::uint64_t PreparedAdjustment::end_frame() const {
    return end_frame_;
}

std::uint64_t PreparedAdjustment::trim_start_millis() const {
    return trim_start_millis_;
}

std::uint64_t PreparedAdjustment::trim_end_millis() const {
    return trim_end_millis_;
}

std::uint64_t PreparedAdjustment::clamp_seek_millis(std::uint64_t millis) const {
    return std::clamp(millis, trim_start_millis_, trim_end_millis_);
}

float PreparedAdjustment::amplitude_at(std::uint64_t source_frame) const {
    if (source_frame < start_frame_ || source_frame >= end_frame_) {
        return 0.0F;
    }

    float envelope = 1.0F;
    if (fade_in_frames_ > 0 && source_frame < start_frame_ + fade_in_frames_) {
        const float progress =
            static_cast<float>(source_frame - start_frame_) / static_cast<float>(fade_in_frames_);
        envelope = evaluate_curve(progress, fade_in_curve_);
    }
    if (fade_out_frames_ > 0 && source_frame >= end_frame_ - fade_out_frames_) {
        const float progress =
            static_cast<float>(end_frame_ - source_frame) / static_cast<float>(fade_out_frames_);
        const float fade_out = evaluate_curve(progress, fade_out_curve_);
        envelope = std::min(envelope, fade_out);
    }
    return gain_amplitude_ * envelope;
}

} // namespace echo::audio
