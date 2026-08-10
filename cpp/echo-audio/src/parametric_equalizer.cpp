#include "echo/audio/parametric_equalizer.hpp"

#include <algorithm>
#include <cmath>
#include <complex>
#include <stdexcept>

namespace echo::audio {
namespace {

constexpr double kPi = 3.14159265358979323846;
constexpr std::int16_t kMinimumGainCentibels = -1200;
constexpr std::int16_t kMaximumGainCentibels = 1200;
constexpr std::uint16_t kMinimumFrequencyHertz = 20;
constexpr std::uint16_t kMaximumFrequencyHertz = 20000;
constexpr std::uint16_t kMinimumQHundredths = 10;
constexpr std::uint16_t kMaximumQHundredths = 2000;
constexpr std::uint32_t kTransitionMillis = 30;

double amplitude(std::int16_t gain_centibels) {
    return std::pow(10.0, static_cast<double>(gain_centibels) / 4000.0);
}

bool valid_filter(EqualizerFilterKind filter) {
    switch (filter) {
    case EqualizerFilterKind::Bell:
    case EqualizerFilterKind::LowShelf:
    case EqualizerFilterKind::HighShelf:
    case EqualizerFilterKind::Notch:
        return true;
    }
    return false;
}

} // namespace

ParametricEqualizer::Section
ParametricEqualizer::normalized(Coefficients coefficients, std::size_t channel_count) {
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

ParametricEqualizer::Section ParametricEqualizer::prepare_section(
    ParametricEqualizerBand band,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (!band.enabled
        || (band.filter_kind != EqualizerFilterKind::Notch && band.gain_centibels == 0)) {
        return normalized({1.0, 0.0, 0.0, 1.0, 0.0, 0.0}, channel_count);
    }
    const double omega =
        2.0 * kPi * static_cast<double>(band.frequency_hertz) / static_cast<double>(sample_rate);
    const double cosine = std::cos(omega);
    const double sine = std::sin(omega);
    const double quality = static_cast<double>(band.q_hundredths) / 100.0;
    const double alpha = sine / (2.0 * quality);
    const double a = amplitude(band.gain_centibels);
    switch (band.filter_kind) {
    case EqualizerFilterKind::Bell:
        return normalized(
            {1.0 + alpha * a,
             -2.0 * cosine,
             1.0 - alpha * a,
             1.0 + alpha / a,
             -2.0 * cosine,
             1.0 - alpha / a},
            channel_count
        );
    case EqualizerFilterKind::LowShelf: {
        const double beta = 2.0 * std::sqrt(a) * alpha;
        return normalized(
            {a * ((a + 1.0) - (a - 1.0) * cosine + beta),
             2.0 * a * ((a - 1.0) - (a + 1.0) * cosine),
             a * ((a + 1.0) - (a - 1.0) * cosine - beta),
             (a + 1.0) + (a - 1.0) * cosine + beta,
             -2.0 * ((a - 1.0) + (a + 1.0) * cosine),
             (a + 1.0) + (a - 1.0) * cosine - beta},
            channel_count
        );
    }
    case EqualizerFilterKind::HighShelf: {
        const double beta = 2.0 * std::sqrt(a) * alpha;
        return normalized(
            {a * ((a + 1.0) + (a - 1.0) * cosine + beta),
             -2.0 * a * ((a - 1.0) + (a + 1.0) * cosine),
             a * ((a + 1.0) + (a - 1.0) * cosine - beta),
             (a + 1.0) - (a - 1.0) * cosine + beta,
             2.0 * ((a - 1.0) - (a + 1.0) * cosine),
             (a + 1.0) - (a - 1.0) * cosine - beta},
            channel_count
        );
    }
    case EqualizerFilterKind::Notch:
        return normalized(
            {1.0, -2.0 * cosine, 1.0, 1.0 + alpha, -2.0 * cosine, 1.0 - alpha},
            channel_count
        );
    }
    throw std::invalid_argument("equalizer filter is outside the supported range");
}

float ParametricEqualizer::Section::process(float sample, std::size_t channel) {
    State& state = states.at(channel);
    const float output = b0 * sample + state.z1;
    state.z1 = b1 * sample - a1 * output + state.z2;
    state.z2 = b2 * sample - a2 * output;
    return output;
}

void ParametricEqualizer::Section::reset() {
    std::fill(states.begin(), states.end(), State{});
}

float ParametricEqualizer::Bank::process(float sample, std::size_t channel) {
    if (bypassed) {
        return sample;
    }
    float output = sample;
    for (Section& section : sections) {
        output = section.process(output, channel);
    }
    return output;
}

void ParametricEqualizer::Bank::reset() {
    for (Section& section : sections) {
        section.reset();
    }
}

void ParametricEqualizer::validate(
    ParametricEqualizerAdjustment adjustment,
    std::uint32_t sample_rate
) {
    for (const ParametricEqualizerBand& band : adjustment.bands) {
        if (!valid_filter(band.filter_kind) || band.frequency_hertz < kMinimumFrequencyHertz
            || band.frequency_hertz > kMaximumFrequencyHertz
            || band.frequency_hertz >= sample_rate / 2 || band.q_hundredths < kMinimumQHundredths
            || band.q_hundredths > kMaximumQHundredths
            || band.gain_centibels < kMinimumGainCentibels
            || band.gain_centibels > kMaximumGainCentibels) {
            throw std::invalid_argument("equalizer band is outside the supported range");
        }
    }
}

bool ParametricEqualizer::same(
    ParametricEqualizerAdjustment left,
    ParametricEqualizerAdjustment right
) {
    for (std::size_t index = 0; index < left.bands.size(); ++index) {
        const auto& a = left.bands[index];
        const auto& b = right.bands[index];
        if (a.enabled != b.enabled || a.filter_kind != b.filter_kind
            || a.frequency_hertz != b.frequency_hertz || a.q_hundredths != b.q_hundredths
            || a.gain_centibels != b.gain_centibels) {
            return false;
        }
    }
    return true;
}

ParametricEqualizer::Bank ParametricEqualizer::prepare(
    ParametricEqualizerAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    validate(adjustment, sample_rate);
    Bank bank;
    bank.adjustment = adjustment;
    bank.bypassed = true;
    for (std::size_t index = 0; index < adjustment.bands.size(); ++index) {
        const auto band = adjustment.bands[index];
        bank.sections[index] = prepare_section(band, sample_rate, channel_count);
        bank.bypassed =
            bank.bypassed
            && (!band.enabled
                || (band.filter_kind != EqualizerFilterKind::Notch && band.gain_centibels == 0));
    }
    return bank;
}

ParametricEqualizer::ParametricEqualizer(
    ParametricEqualizerAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : sample_rate_(sample_rate), channel_count_(channel_count) {
    if (sample_rate == 0 || channel_count == 0) {
        throw std::invalid_argument("equalizer requires valid audio dimensions");
    }
    transition_total_frames_ =
        std::max<std::size_t>(1, static_cast<std::size_t>(sample_rate) * kTransitionMillis / 1000);
    current_ = prepare(adjustment, sample_rate_, channel_count_);
}

void ParametricEqualizer::begin_transition(ParametricEqualizerAdjustment adjustment) {
    if (same(current_.adjustment, adjustment)) {
        return;
    }
    next_ = prepare(adjustment, sample_rate_, channel_count_);
    transition_frame_ = 0;
}

void ParametricEqualizer::transition_to(ParametricEqualizerAdjustment adjustment) {
    validate(adjustment, sample_rate_);
    if (!next_.has_value()) {
        begin_transition(adjustment);
    } else if (same(next_->adjustment, adjustment)) {
        pending_.reset();
    } else {
        pending_ = adjustment;
    }
}

float ParametricEqualizer::process_sample(float sample, std::size_t channel) {
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
    if (channel + 1 == channel_count_ && ++transition_frame_ >= transition_total_frames_) {
        current_ = std::move(*next_);
        next_.reset();
        transition_frame_ = 0;
        if (pending_.has_value()) {
            const ParametricEqualizerAdjustment pending = *pending_;
            pending_.reset();
            begin_transition(pending);
        }
    }
    return output;
}

void ParametricEqualizer::reset() {
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

bool ParametricEqualizer::is_bypassed() const {
    return current_.bypassed && !next_.has_value() && !pending_.has_value();
}

double ParametricEqualizer::response_decibels(
    ParametricEqualizerAdjustment adjustment,
    std::uint32_t sample_rate,
    double frequency_hertz
) {
    validate(adjustment, sample_rate);
    const double omega = 2.0 * kPi * frequency_hertz / static_cast<double>(sample_rate);
    const std::complex<double> z1 = std::polar(1.0, -omega);
    const std::complex<double> z2 = z1 * z1;
    double magnitude = 1.0;
    for (const ParametricEqualizerBand band : adjustment.bands) {
        const Section section = prepare_section(band, sample_rate, 1);
        const std::complex<double> numerator = static_cast<double>(section.b0)
                                               + static_cast<double>(section.b1) * z1
                                               + static_cast<double>(section.b2) * z2;
        const std::complex<double> denominator =
            1.0 + static_cast<double>(section.a1) * z1 + static_cast<double>(section.a2) * z2;
        magnitude *= std::abs(numerator / denominator);
    }
    return 20.0 * std::log10(std::max(magnitude, 1.0e-9));
}

} // namespace echo::audio
