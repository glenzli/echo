#include "echo/audio/three_band_equalizer.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr double kPi = 3.14159265358979323846;
constexpr double kLowShelfHertz = 120.0;
constexpr double kMidPeakHertz = 1000.0;
constexpr double kMidQuality = 0.8;
constexpr double kHighShelfHertz = 8000.0;
constexpr std::int16_t kMinimumGainCentibels = -1200;
constexpr std::int16_t kMaximumGainCentibels = 1200;
constexpr std::uint32_t kTransitionMillis = 30;

double amplitude(std::int16_t gain_centibels) {
    return std::pow(10.0, static_cast<double>(gain_centibels) / 4000.0);
}

} // namespace

ThreeBandEqualizer::Section
ThreeBandEqualizer::normalized(Coefficients coefficients, std::size_t channel_count) {
    if (std::abs(coefficients.a0) < 1.0e-12) {
        throw std::invalid_argument("equalizer coefficient normalization failed");
    }
    const double inverse_a0 = 1.0 / coefficients.a0;
    Section section;
    section.b0 = static_cast<float>(coefficients.b0 * inverse_a0);
    section.b1 = static_cast<float>(coefficients.b1 * inverse_a0);
    section.b2 = static_cast<float>(coefficients.b2 * inverse_a0);
    section.a1 = static_cast<float>(coefficients.a1 * inverse_a0);
    section.a2 = static_cast<float>(coefficients.a2 * inverse_a0);
    section.states.resize(channel_count);
    return section;
}

ThreeBandEqualizer::Section ThreeBandEqualizer::low_shelf(
    std::int16_t gain_centibels,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (gain_centibels == 0) {
        return normalized({1.0, 0.0, 0.0, 1.0, 0.0, 0.0}, channel_count);
    }
    const double a = amplitude(gain_centibels);
    const double omega = 2.0 * kPi * kLowShelfHertz / static_cast<double>(sample_rate);
    const double cosine = std::cos(omega);
    const double sine = std::sin(omega);
    const double alpha = sine / std::sqrt(2.0);
    const double beta = 2.0 * std::sqrt(a) * alpha;
    return normalized(
        {
            a * ((a + 1.0) - (a - 1.0) * cosine + beta),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cosine),
            a * ((a + 1.0) - (a - 1.0) * cosine - beta),
            (a + 1.0) + (a - 1.0) * cosine + beta,
            -2.0 * ((a - 1.0) + (a + 1.0) * cosine),
            (a + 1.0) + (a - 1.0) * cosine - beta,
        },
        channel_count
    );
}

ThreeBandEqualizer::Section ThreeBandEqualizer::peaking(
    std::int16_t gain_centibels,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (gain_centibels == 0) {
        return normalized({1.0, 0.0, 0.0, 1.0, 0.0, 0.0}, channel_count);
    }
    const double a = amplitude(gain_centibels);
    const double omega = 2.0 * kPi * kMidPeakHertz / static_cast<double>(sample_rate);
    const double cosine = std::cos(omega);
    const double alpha = std::sin(omega) / (2.0 * kMidQuality);
    return normalized(
        {
            1.0 + alpha * a,
            -2.0 * cosine,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cosine,
            1.0 - alpha / a,
        },
        channel_count
    );
}

ThreeBandEqualizer::Section ThreeBandEqualizer::high_shelf(
    std::int16_t gain_centibels,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (gain_centibels == 0) {
        return normalized({1.0, 0.0, 0.0, 1.0, 0.0, 0.0}, channel_count);
    }
    const double a = amplitude(gain_centibels);
    const double omega = 2.0 * kPi * kHighShelfHertz / static_cast<double>(sample_rate);
    const double cosine = std::cos(omega);
    const double sine = std::sin(omega);
    const double alpha = sine / std::sqrt(2.0);
    const double beta = 2.0 * std::sqrt(a) * alpha;
    return normalized(
        {
            a * ((a + 1.0) + (a - 1.0) * cosine + beta),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cosine),
            a * ((a + 1.0) + (a - 1.0) * cosine - beta),
            (a + 1.0) - (a - 1.0) * cosine + beta,
            2.0 * ((a - 1.0) - (a + 1.0) * cosine),
            (a + 1.0) - (a - 1.0) * cosine - beta,
        },
        channel_count
    );
}

float ThreeBandEqualizer::Section::process(float sample, std::size_t channel) {
    State& state = states.at(channel);
    const float output = b0 * sample + state.z1;
    state.z1 = b1 * sample - a1 * output + state.z2;
    state.z2 = b2 * sample - a2 * output;
    return output;
}

void ThreeBandEqualizer::Section::reset() {
    std::fill(states.begin(), states.end(), State{});
}

float ThreeBandEqualizer::Bank::process(float sample, std::size_t channel) {
    if (bypassed) {
        return sample;
    }
    return high.process(mid.process(low.process(sample, channel), channel), channel);
}

void ThreeBandEqualizer::Bank::reset() {
    low.reset();
    mid.reset();
    high.reset();
}

void ThreeBandEqualizer::validate(ThreeBandEqualizerAdjustment adjustment) {
    for (const std::int16_t gain : {
             adjustment.low_gain_centibels,
             adjustment.mid_gain_centibels,
             adjustment.high_gain_centibels,
         }) {
        if (gain < kMinimumGainCentibels || gain > kMaximumGainCentibels) {
            throw std::invalid_argument("equalizer gain is outside the supported range");
        }
    }
}

bool ThreeBandEqualizer::same(
    ThreeBandEqualizerAdjustment left,
    ThreeBandEqualizerAdjustment right
) {
    return left.low_gain_centibels == right.low_gain_centibels
           && left.mid_gain_centibels == right.mid_gain_centibels
           && left.high_gain_centibels == right.high_gain_centibels;
}

ThreeBandEqualizer::Bank ThreeBandEqualizer::prepare(
    ThreeBandEqualizerAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    validate(adjustment);
    return {
        .adjustment = adjustment,
        .low = low_shelf(adjustment.low_gain_centibels, sample_rate, channel_count),
        .mid = peaking(adjustment.mid_gain_centibels, sample_rate, channel_count),
        .high = high_shelf(adjustment.high_gain_centibels, sample_rate, channel_count),
        .bypassed = adjustment.low_gain_centibels == 0 && adjustment.mid_gain_centibels == 0
                    && adjustment.high_gain_centibels == 0,
    };
}

ThreeBandEqualizer::ThreeBandEqualizer(
    ThreeBandEqualizerAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (sample_rate == 0 || channel_count == 0
        || kHighShelfHertz >= static_cast<double>(sample_rate) / 2.0) {
        throw std::invalid_argument("equalizer requires valid audio dimensions");
    }
    sample_rate_ = sample_rate;
    channel_count_ = channel_count;
    transition_total_frames_ =
        std::max<std::size_t>(1, static_cast<std::size_t>(sample_rate) * kTransitionMillis / 1000);
    current_ = prepare(adjustment, sample_rate_, channel_count_);
}

void ThreeBandEqualizer::begin_transition(ThreeBandEqualizerAdjustment adjustment) {
    if (same(current_.adjustment, adjustment)) {
        return;
    }
    next_ = prepare(adjustment, sample_rate_, channel_count_);
    transition_frame_ = 0;
}

void ThreeBandEqualizer::transition_to(ThreeBandEqualizerAdjustment adjustment) {
    validate(adjustment);
    if (!next_.has_value()) {
        begin_transition(adjustment);
        return;
    }
    if (same(next_->adjustment, adjustment)) {
        pending_.reset();
        return;
    }
    pending_ = adjustment;
}

float ThreeBandEqualizer::process_sample(float sample, std::size_t channel) {
    if (channel >= channel_count_) {
        throw std::out_of_range("equalizer channel is outside the prepared layout");
    }
    const float current_output = current_.process(sample, channel);
    if (!next_.has_value()) {
        return current_output;
    }

    const float next_output = next_->process(sample, channel);
    const float progress = std::min(
        1.0F,
        static_cast<float>(transition_frame_ + 1) / static_cast<float>(transition_total_frames_)
    );
    const float output = current_output + (next_output - current_output) * progress;

    if (channel + 1 == channel_count_) {
        ++transition_frame_;
        if (transition_frame_ >= transition_total_frames_) {
            current_ = std::move(*next_);
            next_.reset();
            transition_frame_ = 0;
            if (pending_.has_value()) {
                const ThreeBandEqualizerAdjustment pending = *pending_;
                pending_.reset();
                begin_transition(pending);
            }
        }
    }
    return output;
}

void ThreeBandEqualizer::reset() {
    if (pending_.has_value()) {
        current_ = prepare(*pending_, sample_rate_, channel_count_);
    } else if (next_.has_value()) {
        current_ = std::move(*next_);
    }
    next_.reset();
    pending_.reset();
    transition_frame_ = 0;
    current_.reset();
}

bool ThreeBandEqualizer::is_bypassed() const {
    return current_.bypassed && !next_.has_value() && !pending_.has_value();
}

} // namespace echo::audio
