#include "echo/audio/granular_vfx_processor.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kCanonicalSampleRate = 48000;
constexpr std::size_t kMaximumChannels = 2;
constexpr std::size_t kVoiceCount = 16;
constexpr std::uint16_t kMinimumGrainMillis = 20;
constexpr std::uint16_t kMaximumGrainMillis = 250;
constexpr std::uint16_t kMinimumDensityTenthsHertz = 10;
constexpr std::uint16_t kMaximumDensityTenthsHertz = 400;
constexpr std::uint16_t kMaximumLookbackMillis = 1500;
constexpr std::uint16_t kMaximumScatterMillis = 750;
constexpr std::int16_t kMinimumPitchCents = -1200;
constexpr std::int16_t kMaximumPitchCents = 1200;
constexpr std::uint16_t kHistoryMillis = 2000;
constexpr std::size_t kHistoryFrames =
    static_cast<std::size_t>(kCanonicalSampleRate) * kHistoryMillis / 1000U;
constexpr std::uint64_t kDensityDenominator =
    static_cast<std::uint64_t>(kCanonicalSampleRate) * 10U;
constexpr float kMaximumSignal = 16.0F;

float finite_bounded(float value) noexcept {
    return std::clamp(std::isfinite(value) ? value : 0.0F, -kMaximumSignal, kMaximumSignal);
}

double pitch_ratio(std::int16_t pitch_cents) noexcept {
    return std::exp2(static_cast<double>(pitch_cents) / 1200.0);
}

void validate(
    GranularVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    const double required_history_millis =
        static_cast<double>(adjustment.lookback_millis)
        + static_cast<double>(adjustment.scatter_millis)
        + static_cast<double>(adjustment.grain_millis)
              * std::max(1.0, pitch_ratio(adjustment.pitch_cents));
    if (sample_rate != kCanonicalSampleRate || channel_count == 0
        || channel_count > kMaximumChannels || adjustment.mix_percent > 100
        || adjustment.grain_millis < kMinimumGrainMillis
        || adjustment.grain_millis > kMaximumGrainMillis
        || adjustment.density_tenths_hertz < kMinimumDensityTenthsHertz
        || adjustment.density_tenths_hertz > kMaximumDensityTenthsHertz
        || adjustment.lookback_millis > kMaximumLookbackMillis
        || adjustment.scatter_millis > kMaximumScatterMillis
        || adjustment.pitch_cents < kMinimumPitchCents
        || adjustment.pitch_cents > kMaximumPitchCents || adjustment.stereo_spread_percent > 100
        || required_history_millis > static_cast<double>(kHistoryMillis)) {
        throw std::invalid_argument("granular VFX parameters are outside the supported range");
    }
}

GranularVfxAdjustment
validated(GranularVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) {
    validate(adjustment, sample_rate, channel_count);
    return adjustment;
}

class LinearRamp {
  public:
    void reset(float value) noexcept {
        current_ = value;
        target_ = value;
        remaining_ = 0;
    }

    void set_target(float value, std::size_t frame_count) noexcept {
        target_ = value;
        remaining_ = current_ == target_ ? 0 : frame_count;
    }

    [[nodiscard]] float next() noexcept {
        if (remaining_ != 0) {
            current_ += (target_ - current_) / static_cast<float>(remaining_);
            --remaining_;
            if (remaining_ == 0) {
                current_ = target_;
            }
        }
        return current_;
    }

  private:
    float current_ = 0.0F;
    float target_ = 0.0F;
    std::size_t remaining_ = 0;
};

std::uint64_t mixed_hash(std::uint64_t value) noexcept {
    value += 0x9E3779B97F4A7C15ULL;
    value = (value ^ (value >> 30U)) * 0xBF58476D1CE4E5B9ULL;
    value = (value ^ (value >> 27U)) * 0x94D049BB133111EBULL;
    return value ^ (value >> 31U);
}

float random_unit(
    std::uint32_t seed,
    std::uint64_t absolute_frame,
    std::uint64_t grain_ordinal,
    std::uint64_t stream
) noexcept {
    const std::uint64_t hash = mixed_hash(
        static_cast<std::uint64_t>(seed) ^ mixed_hash(absolute_frame)
        ^ mixed_hash(grain_ordinal + 0xD1B54A32D192ED03ULL * stream)
    );
    constexpr double scale = 1.0 / static_cast<double>(1ULL << 24U);
    return static_cast<float>(static_cast<double>(hash >> 40U) * scale);
}

} // namespace

class GranularVfxProcessor::Impl {
  public:
    Impl(GranularVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count),
        authored_(validated(adjustment, sample_rate, channel_count)),
        history_(kHistoryFrames * channel_count, 0.0F) {
        smoothing_frames_ = std::max<std::size_t>(1, sample_rate_ / 50U);
        reset_parameters();
    }

    void update(GranularVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        enabled_.set_target(adjustment.enabled ? 1.0F : 0.0F, smoothing_frames_);
        mix_.set_target(static_cast<float>(adjustment.mix_percent) / 100.0F, smoothing_frames_);
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("granular VFX channel layout changed");
        }

        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            std::array<float, kMaximumChannels> dry{};
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                dry[channel] = samples[base + channel];
                history_[write_cursor_ * channel_count_ + channel] = finite_bounded(dry[channel]);
            }
            write_cursor_ = (write_cursor_ + 1U) % kHistoryFrames;
            available_frames_ = std::min(available_frames_ + 1U, kHistoryFrames);

            schedule_grain_event();

            std::array<float, kMaximumChannels> wet{};
            for (Voice& voice : voices_) {
                if (!voice.active) {
                    continue;
                }
                const float window =
                    voice.frame_count == 1
                        ? 1.0F
                        : 0.5F
                              - 0.5F
                                    * std::cos(
                                        2.0F * std::numbers::pi_v<float>
                                        * static_cast<float>(voice.cursor)
                                        / static_cast<float>(voice.frame_count - 1U)
                                    );
                for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                    const float gain = channel == 0 ? voice.left_gain : voice.right_gain;
                    wet[channel] += sample_at_age(voice.source_age, channel) * gain * window
                                    * voice.normalization;
                }
                voice.source_age += voice.source_age_step;
                ++voice.cursor;
                if (voice.cursor == voice.frame_count) {
                    voice.active = false;
                }
            }

            const float wet_mix = enabled_.next() * mix_.next();
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                if (wet_mix == 0.0F) {
                    samples[base + channel] = dry[channel];
                } else {
                    samples[base + channel] = std::clamp(
                        std::lerp(finite_bounded(dry[channel]), wet[channel], wet_mix),
                        -kMaximumSignal,
                        kMaximumSignal
                    );
                }
            }
            ++absolute_frame_;
        }
    }

    void reset(std::uint64_t timeline_frame) noexcept {
        std::fill(history_.begin(), history_.end(), 0.0F);
        for (Voice& voice : voices_) {
            voice = {};
        }
        write_cursor_ = 0;
        available_frames_ = 0;
        const std::uint64_t density = authored_.density_tenths_hertz;
        const std::uint64_t whole_periods = timeline_frame / kDensityDenominator;
        const std::uint64_t remaining_frames = timeline_frame % kDensityDenominator;
        const std::uint64_t remaining_product = remaining_frames * density;
        density_phase_ = remaining_product % kDensityDenominator;
        // Unsigned wrap is intentional: the hash consumes the stable low 64
        // bits of the absolute event ordinal even at impractically long times.
        grain_ordinal_ = whole_periods * density + remaining_product / kDensityDenominator;
        absolute_frame_ = timeline_frame;
        reset_parameters();
    }

    [[nodiscard]] bool is_bypassed() const noexcept {
        return !authored_.enabled || authored_.mix_percent == 0;
    }

    [[nodiscard]] GranularVfxAdjustment adjustment() const noexcept {
        return authored_;
    }

  private:
    struct Voice {
        double source_age = 0.0;
        double source_age_step = 0.0;
        float left_gain = 1.0F;
        float right_gain = 1.0F;
        float normalization = 1.0F;
        std::size_t frame_count = 0;
        std::size_t cursor = 0;
        bool active = false;
    };

    void reset_parameters() noexcept {
        enabled_.reset(authored_.enabled ? 1.0F : 0.0F);
        mix_.reset(static_cast<float>(authored_.mix_percent) / 100.0F);
    }

    void schedule_grain_event() noexcept {
        density_phase_ += authored_.density_tenths_hertz;
        if (density_phase_ < kDensityDenominator) {
            return;
        }
        density_phase_ -= kDensityDenominator;
        const std::uint64_t ordinal = grain_ordinal_++;
        if (!authored_.enabled || authored_.mix_percent == 0) {
            return;
        }
        for (Voice& voice : voices_) {
            if (!voice.active) {
                spawn(voice, ordinal);
                return;
            }
        }
    }

    void spawn(Voice& voice, std::uint64_t ordinal) noexcept {
        const std::size_t grain_frames = static_cast<std::size_t>(
            static_cast<std::uint64_t>(authored_.grain_millis) * sample_rate_ / 1000U
        );
        const double ratio = pitch_ratio(authored_.pitch_cents);
        const float centered_scatter =
            2.0F * random_unit(authored_.random_seed, absolute_frame_, ordinal, 1U) - 1.0F;
        const double end_age = std::max(
            0.0,
            static_cast<double>(authored_.lookback_millis) * static_cast<double>(sample_rate_)
                    / 1000.0
                + static_cast<double>(centered_scatter)
                      * static_cast<double>(authored_.scatter_millis)
                      * static_cast<double>(sample_rate_) / 1000.0
        );
        const double maximum_age =
            end_age + static_cast<double>(grain_frames - 1U) * std::max(1.0, ratio) + 1.0;
        if (maximum_age >= static_cast<double>(available_frames_)) {
            return;
        }

        float left_gain = 1.0F;
        float right_gain = 1.0F;
        if (channel_count_ == 2) {
            const float pan_random =
                2.0F * random_unit(authored_.random_seed, absolute_frame_, ordinal, 2U) - 1.0F;
            const float spread = static_cast<float>(authored_.stereo_spread_percent) / 100.0F;
            const float angle = std::numbers::pi_v<float> * 0.25F
                                + pan_random * spread * std::numbers::pi_v<float> * 0.25F;
            left_gain = std::numbers::sqrt2_v<float> * std::cos(angle);
            right_gain = std::numbers::sqrt2_v<float> * std::sin(angle);
        }
        const float overlap = std::max(
            1.0F,
            static_cast<float>(authored_.density_tenths_hertz)
                * static_cast<float>(authored_.grain_millis) / 10000.0F
        );
        voice.source_age = end_age + static_cast<double>(grain_frames - 1U) * ratio;
        voice.source_age_step = 1.0 - ratio;
        voice.left_gain = left_gain;
        voice.right_gain = right_gain;
        voice.normalization = 1.0F / std::sqrt(overlap);
        voice.frame_count = grain_frames;
        voice.cursor = 0;
        voice.active = true;
    }

    [[nodiscard]] float sample_at_age(double age, std::size_t channel) const noexcept {
        const std::size_t younger_age = static_cast<std::size_t>(age);
        const std::size_t older_age = younger_age + 1U;
        const float fraction = static_cast<float>(age - static_cast<double>(younger_age));
        const float younger = history_sample(younger_age, channel);
        const float older = history_sample(older_age, channel);
        return std::lerp(younger, older, fraction);
    }

    [[nodiscard]] float history_sample(std::size_t age, std::size_t channel) const noexcept {
        const std::size_t newest = write_cursor_ == 0 ? kHistoryFrames - 1U : write_cursor_ - 1U;
        const std::size_t frame = (newest + kHistoryFrames - age) % kHistoryFrames;
        return history_[frame * channel_count_ + channel];
    }

    std::uint32_t sample_rate_;
    std::size_t channel_count_;
    GranularVfxAdjustment authored_;
    std::size_t smoothing_frames_ = 1;
    std::vector<float> history_;
    std::array<Voice, kVoiceCount> voices_{};
    LinearRamp enabled_;
    LinearRamp mix_;
    std::size_t write_cursor_ = 0;
    std::size_t available_frames_ = 0;
    std::uint64_t density_phase_ = 0;
    std::uint64_t grain_ordinal_ = 0;
    std::uint64_t absolute_frame_ = 0;
};

GranularVfxProcessor::GranularVfxProcessor(
    GranularVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

GranularVfxProcessor::~GranularVfxProcessor() = default;

void GranularVfxProcessor::update(GranularVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

void GranularVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void GranularVfxProcessor::reset(std::uint64_t timeline_frame) noexcept {
    impl_->reset(timeline_frame);
}

bool GranularVfxProcessor::is_bypassed() const noexcept {
    return impl_->is_bypassed();
}

GranularVfxAdjustment GranularVfxProcessor::adjustment() const noexcept {
    return impl_->adjustment();
}

} // namespace echo::audio
