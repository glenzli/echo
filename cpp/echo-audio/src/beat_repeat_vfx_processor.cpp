#include "echo/audio/beat_repeat_vfx_processor.hpp"

#include <algorithm>
#include <cmath>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kMinimumSampleRate = 8000;
constexpr std::uint32_t kMaximumSampleRate = 192000;
constexpr std::uint16_t kMaximumSliceMillis = 500;
constexpr std::uint8_t kMaximumRepeatCount = 4;

void validate(BeatRepeatVfxParameters parameters, std::uint32_t sample_rate, std::size_t channels) {
    if (parameters.mix_percent > 100 || parameters.slice_millis < 30
        || parameters.slice_millis > kMaximumSliceMillis || parameters.repeat_count == 0
        || parameters.repeat_count > kMaximumRepeatCount || sample_rate < kMinimumSampleRate
        || sample_rate > kMaximumSampleRate || channels == 0 || channels > 2) {
        throw std::invalid_argument("beat repeat VFX parameters are outside the supported range");
    }
}

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

} // namespace

class BeatRepeatVfxProcessor::Impl {
  public:
    Impl(BeatRepeatVfxParameters parameters, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), parameters_(parameters),
        latency_frames_(
            static_cast<std::size_t>(sample_rate) * kMaximumSliceMillis * kMaximumRepeatCount
            / 1000U
        ),
        history_frames_(latency_frames_ * 2), history_(history_frames_ * channel_count, 0.0F) {
        validate(parameters, sample_rate, channel_count);
    }

    void update(BeatRepeatVfxParameters parameters) {
        validate(parameters, sample_rate_, channel_count_);
        parameters_ = parameters;
    }

    void process(float* samples, std::size_t frames, std::size_t channels) {
        if ((samples == nullptr && frames != 0) || channels != channel_count_) {
            throw std::invalid_argument("beat repeat VFX channel layout changed");
        }
        const std::size_t slice_frames = std::max<std::size_t>(
            1,
            static_cast<std::size_t>(parameters_.slice_millis) * sample_rate_ / 1000U
        );
        const std::size_t block_frames = slice_frames * parameters_.repeat_count;
        const float wet_mix =
            parameters_.enabled ? static_cast<float>(parameters_.mix_percent) / 100.0F : 0.0F;
        for (std::size_t frame = 0; frame < frames; ++frame) {
            const std::uint64_t audible_frame =
                frame_cursor_ < latency_frames_ ? 0 : frame_cursor_ - latency_frames_;
            const std::size_t phase = static_cast<std::size_t>(audible_frame % block_frames);
            const std::size_t source_offset = parameters_.reverse
                                                  ? slice_frames - 1 - (phase % slice_frames)
                                                  : phase % slice_frames;
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const std::size_t sample = frame * channel_count_ + channel;
                const float dry = finite(samples[sample]);
                const std::size_t delayed_dry_index =
                    (write_index_ + history_frames_ - latency_frames_) % history_frames_;
                const float delayed_dry = history_[delayed_dry_index * channel_count_ + channel];
                const std::int64_t relative_offset = static_cast<std::int64_t>(source_offset)
                                                     - static_cast<std::int64_t>(phase)
                                                     - static_cast<std::int64_t>(latency_frames_);
                const std::int64_t unwrapped_index =
                    static_cast<std::int64_t>(write_index_) + relative_offset;
                const std::size_t read_index = static_cast<std::size_t>(
                    (unwrapped_index % static_cast<std::int64_t>(history_frames_)
                     + static_cast<std::int64_t>(history_frames_))
                    % static_cast<std::int64_t>(history_frames_)
                );
                const float wet = history_[read_index * channel_count_ + channel];
                history_[write_index_ * channel_count_ + channel] = dry;
                samples[sample] = finite(delayed_dry + wet_mix * (wet - delayed_dry));
            }
            write_index_ = (write_index_ + 1) % history_frames_;
            ++frame_cursor_;
        }
    }

    void reset() {
        std::fill(history_.begin(), history_.end(), 0.0F);
        write_index_ = 0;
        frame_cursor_ = 0;
    }

    [[nodiscard]] bool bypassed() const noexcept {
        return !parameters_.enabled;
    }
    [[nodiscard]] BeatRepeatVfxParameters parameters() const noexcept {
        return parameters_;
    }
    [[nodiscard]] std::size_t latency_frames() const noexcept {
        return latency_frames_;
    }

  private:
    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    BeatRepeatVfxParameters parameters_{};
    std::size_t latency_frames_ = 0;
    std::size_t history_frames_ = 0;
    std::vector<float> history_;
    std::size_t write_index_ = 0;
    std::uint64_t frame_cursor_ = 0;
};

BeatRepeatVfxProcessor::BeatRepeatVfxProcessor(
    BeatRepeatVfxParameters parameters,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(parameters, sample_rate, channel_count)) {}

BeatRepeatVfxProcessor::~BeatRepeatVfxProcessor() = default;
void BeatRepeatVfxProcessor::update(BeatRepeatVfxParameters parameters) {
    impl_->update(parameters);
}
void BeatRepeatVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frames,
    std::size_t channels
) {
    impl_->process(samples, frames, channels);
}
void BeatRepeatVfxProcessor::reset() {
    impl_->reset();
}
bool BeatRepeatVfxProcessor::is_bypassed() const noexcept {
    return impl_->bypassed();
}
BeatRepeatVfxParameters BeatRepeatVfxProcessor::parameters() const noexcept {
    return impl_->parameters();
}
std::size_t BeatRepeatVfxProcessor::latency_frames() const noexcept {
    return impl_->latency_frames();
}

} // namespace echo::audio
