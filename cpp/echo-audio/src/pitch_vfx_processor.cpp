#include "echo/audio/pitch_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;
constexpr std::size_t kMaximumChannels = 2;
constexpr float kMinimumDelayMillis = 12.0F;
constexpr float kMaximumDelayMillis = 82.0F;
constexpr float kMaximumSemitones = 12.0F;

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

float semitones_to_ratio(std::int8_t semitones) noexcept {
    return std::exp2(static_cast<float>(semitones) / 12.0F);
}

void validate(PitchVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) {
    if (parameters.mix_percent > 100 || parameters.pitch_semitones < -kMaximumSemitones
        || parameters.pitch_semitones > kMaximumSemitones
        || parameters.harmony_semitones < -kMaximumSemitones
        || parameters.harmony_semitones > kMaximumSemitones || parameters.harmony_mix_percent > 100
        || parameters.formant_colour_semitones < -kMaximumSemitones
        || parameters.formant_colour_semitones > kMaximumSemitones
        || sample_rate < kMinimumSampleRate || sample_rate > kMaximumSampleRate
        || channel_count == 0 || channel_count > kMaximumChannels) {
        throw std::invalid_argument("pitch VFX parameters are outside the supported range");
    }
}

float lerp(float from, float to, float amount) noexcept {
    return from + (to - from) * amount;
}

class Biquad {
  public:
    void band_pass(float frequency_hertz, float quality, std::uint32_t sample_rate) noexcept {
        const float omega =
            2.0F * std::numbers::pi_v<float> * frequency_hertz / static_cast<float>(sample_rate);
        const float alpha = std::sin(omega) / (2.0F * quality);
        const float cosine = std::cos(omega);
        normalize(alpha, 0.0F, alpha, 1.0F + alpha, -2.0F * cosine, 1.0F - alpha);
    }

    [[nodiscard]] float process(float input) noexcept {
        const float output = b0_ * input + z1_;
        z1_ = b1_ * input - a1_ * output + z2_;
        z2_ = b2_ * input - a2_ * output;
        return output;
    }

    void reset() noexcept {
        z1_ = z2_ = 0.0F;
    }

  private:
    void normalize(float b0, float b1, float b2, float a0, float a1, float a2) noexcept {
        b0_ = b0 / a0;
        b1_ = b1 / a0;
        b2_ = b2 / a0;
        a1_ = a1 / a0;
        a2_ = a2 / a0;
    }

    float b0_ = 1.0F;
    float b1_ = 0.0F;
    float b2_ = 0.0F;
    float a1_ = 0.0F;
    float a2_ = 0.0F;
    float z1_ = 0.0F;
    float z2_ = 0.0F;
};

} // namespace

class PitchVfxProcessor::Impl {
  public:
    Impl(PitchVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), parameters_(parameters),
        latency_frames_(
            std::max<std::size_t>(
                1,
                static_cast<std::size_t>(
                    std::ceil(kMinimumDelayMillis * static_cast<float>(sample_rate) / 1000.0F)
                )
            )
        ),
        maximum_delay_frames_(
            std::max<std::size_t>(
                latency_frames_ + 3,
                static_cast<std::size_t>(
                    std::ceil(kMaximumDelayMillis * static_cast<float>(sample_rate) / 1000.0F)
                ) + 3
            )
        ),
        delay_line_(maximum_delay_frames_ * channel_count_, 0.0F) {
        validate(parameters, sample_rate, channel_count);
        configure_formants();
    }

    void update(PitchVfxParameters parameters) {
        validate(parameters, sample_rate_, channel_count_);
        const bool formants_changed =
            parameters.formant_colour_semitones != parameters_.formant_colour_semitones;
        parameters_ = parameters;
        if (formants_changed) {
            configure_formants();
        }
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("pitch VFX channel layout changed");
        }
        const float pitch_ratio = semitones_to_ratio(parameters_.pitch_semitones);
        const float harmony_ratio = semitones_to_ratio(parameters_.harmony_semitones);
        const float wet_mix =
            parameters_.enabled ? static_cast<float>(parameters_.mix_percent) / 100.0F : 0.0F;
        const float harmony_mix = parameters_.harmony_enabled
                                      ? static_cast<float>(parameters_.harmony_mix_percent) / 100.0F
                                      : 0.0F;
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            std::array<float, kMaximumChannels> input{};
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                input[channel] = finite(samples[base + channel]);
                delay_line_[cursor_ * channel_count_ + channel] = input[channel];
            }
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float dry = read_delay(channel, static_cast<float>(latency_frames_));
                const float primary = shifted_sample(channel, pitch_ratio, primary_phase_);
                const float harmony = shifted_sample(channel, harmony_ratio, harmony_phase_);
                const float combined = lerp(primary, harmony, harmony_mix);
                const float coloured = formant_colour(channel, combined);
                samples[base + channel] = finite(lerp(dry, coloured, wet_mix));
            }
            cursor_ = (cursor_ + 1) % maximum_delay_frames_;
            primary_phase_ = advance_phase(primary_phase_, pitch_ratio);
            harmony_phase_ = advance_phase(harmony_phase_, harmony_ratio);
        }
    }

    void reset() noexcept {
        std::fill(delay_line_.begin(), delay_line_.end(), 0.0F);
        for (auto& channel : formants_) {
            for (auto& filter : channel) {
                filter.reset();
            }
        }
        cursor_ = 0;
        primary_phase_ = 0.0F;
        harmony_phase_ = 0.5F;
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !parameters_.enabled;
    }
    [[nodiscard]] PitchVfxParameters parameters() const noexcept {
        return parameters_;
    }
    [[nodiscard]] std::size_t latency_frames() const noexcept {
        return latency_frames_;
    }

  private:
    [[nodiscard]] float read_delay(std::size_t channel, float delay) const noexcept {
        const float clamped =
            std::clamp(delay, 1.0F, static_cast<float>(maximum_delay_frames_ - 2));
        const std::size_t whole = static_cast<std::size_t>(clamped);
        const float fraction = clamped - static_cast<float>(whole);
        const std::size_t newer = (cursor_ + maximum_delay_frames_ - whole) % maximum_delay_frames_;
        const std::size_t older = (newer + maximum_delay_frames_ - 1) % maximum_delay_frames_;
        return lerp(
            delay_line_[newer * channel_count_ + channel],
            delay_line_[older * channel_count_ + channel],
            fraction
        );
    }

    [[nodiscard]] float
    shifted_sample(std::size_t channel, float ratio, float phase) const noexcept {
        if (std::abs(ratio - 1.0F) < 1.0E-6F) {
            return read_delay(channel, static_cast<float>(latency_frames_));
        }
        const float span = static_cast<float>(maximum_delay_frames_ - latency_frames_ - 2);
        const float first_delay = static_cast<float>(latency_frames_) + phase * span;
        const float second_phase = phase < 0.5F ? phase + 0.5F : phase - 0.5F;
        const float second_delay = static_cast<float>(latency_frames_) + second_phase * span;
        const float first_weight = 0.5F - 0.5F * std::cos(2.0F * std::numbers::pi_v<float> * phase);
        return lerp(
            read_delay(channel, second_delay),
            read_delay(channel, first_delay),
            first_weight
        );
    }

    [[nodiscard]] float formant_colour(std::size_t channel, float sample) noexcept {
        if (parameters_.formant_colour_semitones == 0) {
            return sample;
        }
        float resonant = 0.0F;
        for (auto& filter : formants_[channel]) {
            resonant += filter.process(sample);
        }
        return sample + 0.48F * resonant;
    }

    [[nodiscard]] float advance_phase(float phase, float ratio) const noexcept {
        const float span = static_cast<float>(maximum_delay_frames_ - latency_frames_ - 2);
        phase += (1.0F - ratio) / span;
        phase -= std::floor(phase);
        return phase;
    }

    void configure_formants() noexcept {
        const float ratio = semitones_to_ratio(parameters_.formant_colour_semitones);
        constexpr std::array<float, 3> centers{{500.0F, 1500.0F, 2500.0F}};
        for (auto& channel : formants_) {
            for (std::size_t index = 0; index < centers.size(); ++index) {
                const float frequency = std::clamp(
                    centers[index] * ratio,
                    80.0F,
                    static_cast<float>(sample_rate_) * 0.42F
                );
                channel[index].band_pass(frequency, 3.2F, sample_rate_);
                channel[index].reset();
            }
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    PitchVfxParameters parameters_{};
    std::size_t latency_frames_ = 1;
    std::size_t maximum_delay_frames_ = 4;
    std::vector<float> delay_line_;
    std::size_t cursor_ = 0;
    float primary_phase_ = 0.0F;
    float harmony_phase_ = 0.5F;
    std::array<std::array<Biquad, 3>, kMaximumChannels> formants_{};
};

PitchVfxProcessor::PitchVfxProcessor(
    PitchVfxParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(parameters, sample_rate, channel_count)) {}

PitchVfxProcessor::~PitchVfxProcessor() = default;
void PitchVfxProcessor::update(PitchVfxParameters parameters) {
    impl_->update(parameters);
}
void PitchVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frames,
    std::size_t channels
) {
    impl_->process_interleaved(samples, frames, channels);
}
void PitchVfxProcessor::reset() {
    impl_->reset();
}
bool PitchVfxProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}
PitchVfxParameters PitchVfxProcessor::parameters() const noexcept {
    return impl_->parameters();
}
std::size_t PitchVfxProcessor::latency_frames() const noexcept {
    return impl_->latency_frames();
}

} // namespace echo::audio
