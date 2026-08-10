#include "echo/audio/low_cut_filter.hpp"

#include <algorithm>
#include <cmath>
#include <numbers>
#include <stdexcept>

namespace echo::audio {

LowCutFilter::LowCutFilter(
    std::uint16_t cutoff_hertz,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : states_(channel_count) {
    if (sample_rate == 0 || channel_count == 0) {
        throw std::invalid_argument("low-cut filter requires a valid PCM layout");
    }
    if (cutoff_hertz == 0) {
        return;
    }
    if (static_cast<std::uint32_t>(cutoff_hertz) * 2U >= sample_rate) {
        throw std::invalid_argument("low-cut frequency must be below Nyquist");
    }

    constexpr double kButterworthQ = 0.7071067811865476;
    const double omega = 2.0 * std::numbers::pi * static_cast<double>(cutoff_hertz)
                         / static_cast<double>(sample_rate);
    const double cosine = std::cos(omega);
    const double alpha = std::sin(omega) / (2.0 * kButterworthQ);
    const double a_zero = 1.0 + alpha;

    b_zero_ = static_cast<float>(((1.0 + cosine) * 0.5) / a_zero);
    b_one_ = static_cast<float>(-(1.0 + cosine) / a_zero);
    b_two_ = b_zero_;
    a_one_ = static_cast<float>((-2.0 * cosine) / a_zero);
    a_two_ = static_cast<float>((1.0 - alpha) / a_zero);
    enabled_ = true;
}

float LowCutFilter::process_sample(float input, std::size_t channel) {
    if (!enabled_) {
        return input;
    }
    if (channel >= states_.size()) {
        throw std::out_of_range("low-cut channel is outside the prepared layout");
    }
    ChannelState& state = states_[channel];
    const float output = b_zero_ * input + state.delay_one;
    state.delay_one = b_one_ * input - a_one_ * output + state.delay_two;
    state.delay_two = b_two_ * input - a_two_ * output;
    return output;
}

void LowCutFilter::reset() {
    std::fill(states_.begin(), states_.end(), ChannelState{});
}

bool LowCutFilter::enabled() const {
    return enabled_;
}

} // namespace echo::audio
