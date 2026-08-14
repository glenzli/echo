#include "echo/audio/freeze_vfx_processor.hpp"

#include "convolution/signalsmith_audiofft_adapter.hpp"

#include <algorithm>
#include <array>
#include <cmath>
#include <complex>
#include <limits>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kCanonicalSampleRate = 48000;
constexpr std::size_t kMaximumChannels = 2;
constexpr std::size_t kWindowFrames = FreezeVfxProcessor::latency_frames();
constexpr std::size_t kHopFrames = kWindowFrames / 4;
constexpr std::size_t kBinCount = kWindowFrames / 2 + 1;
constexpr std::size_t kOverlapCapacityFrames = kWindowFrames * 3;
constexpr float kMaximumProcessedSample = 16.0F;
constexpr float kTwoPi = 2.0F * std::numbers::pi_v<float>;

float finite(float value) noexcept {
    return std::isfinite(value) ? value : 0.0F;
}

void validate(
    FreezeVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) {
    if (adjustment.mix_percent > 100 || sample_rate != kCanonicalSampleRate || channel_count == 0
        || channel_count > kMaximumChannels) {
        throw std::invalid_argument("freeze VFX parameters are outside the supported range");
    }
}

class LinearRamp {
  public:
    void reset(float value) noexcept {
        current_ = value;
        target_ = value;
        remaining_ = 0;
    }

    void set_target(float value, std::size_t frames) noexcept {
        target_ = value;
        remaining_ = current_ == target_ ? 0 : frames;
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

} // namespace

class FreezeVfxProcessor::Impl {
  public:
    Impl(FreezeVfxAdjustment adjustment, std::uint32_t sample_rate, std::size_t channel_count) :
        sample_rate_(sample_rate), channel_count_(channel_count), authored_(adjustment),
        history_(make_channel_vectors(kWindowFrames)),
        dry_delay_(make_channel_vectors(kWindowFrames)),
        overlap_(make_channel_vectors(kOverlapCapacityFrames)),
        analysis_input_(make_channel_vectors(kWindowFrames)),
        inverse_output_(make_channel_vectors(kWindowFrames)),
        spectrum_real_(make_channel_vectors(kBinCount)),
        spectrum_imaginary_(make_channel_vectors(kBinCount)),
        previous_phase_(make_channel_vectors(kBinCount)),
        captured_magnitude_(make_channel_vectors(kBinCount)),
        synthesis_phase_(make_channel_vectors(kBinCount)),
        phase_advance_(make_channel_vectors(kBinCount)), window_(kWindowFrames, 0.0F) {
        validate(adjustment, sample_rate_, channel_count_);
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            fft_[channel] = std::make_unique<audiofft::AudioFFT>();
            fft_[channel]->init(kWindowFrames);
        }
        for (std::size_t frame = 0; frame < kWindowFrames; ++frame) {
            const float phase =
                kTwoPi * static_cast<float>(frame) / static_cast<float>(kWindowFrames);
            const float hann = 0.5F - 0.5F * std::cos(phase);
            window_[frame] = std::sqrt(std::max(0.0F, hann));
        }
        smoothing_frames_ = sample_rate_ / 50U;
        mix_.reset(target_mix(adjustment));
        capture_gate_.reset(0.0F);
    }

    void update(FreezeVfxAdjustment adjustment) {
        validate(adjustment, sample_rate_, channel_count_);
        authored_ = adjustment;
        mix_.set_target(target_mix(adjustment), smoothing_frames_);
    }

    [[nodiscard]] bool request_capture() noexcept {
        if (has_capture_ || capture_requested_ || input_frames_ < kWindowFrames) {
            return false;
        }
        capture_requested_ = true;
        return true;
    }

    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count) {
        if ((samples == nullptr && frame_count != 0) || channel_count != channel_count_) {
            throw std::invalid_argument("freeze VFX channel layout changed");
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            const std::size_t base = frame * channel_count_;
            std::array<float, kMaximumChannels> input{};
            std::array<float, kMaximumChannels> dry{};
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                input[channel] = finite(samples[base + channel]);
                dry[channel] = dry_delay_[channel][dry_cursor_];
                dry_delay_[channel][dry_cursor_] = input[channel];
                history_[channel][history_cursor_] = input[channel];
            }
            dry_cursor_ = (dry_cursor_ + 1) % kWindowFrames;
            history_cursor_ = (history_cursor_ + 1) % kWindowFrames;
            ++input_frames_;

            bool analyzed = false;
            if (capture_requested_) {
                analyze(true);
                capture_requested_ = false;
                has_capture_ = true;
                // This frame also advances the state machine below. The extra
                // count keeps activation aligned with the first scheduled
                // wet sample exactly `kWindowFrames` frames in the future.
                activation_countdown_ = kWindowFrames + 1;
                next_synthesis_input_frame_ = input_frames_ + kHopFrames;
                schedule_synthesis();
                analyzed = true;
            }
            if (!analyzed && !has_capture_ && input_frames_ >= kWindowFrames
                && (!has_previous_analysis_
                    || input_frames_ - previous_analysis_frame_ >= kHopFrames)) {
                analyze(false);
            }
            if (has_capture_ && input_frames_ == next_synthesis_input_frame_) {
                schedule_synthesis();
                next_synthesis_input_frame_ += kHopFrames;
            }

            if (activation_countdown_ != 0) {
                --activation_countdown_;
                if (activation_countdown_ == 0) {
                    capture_gate_.reset(0.0F);
                    capture_gate_.set_target(1.0F, smoothing_frames_);
                }
            }

            const float effect_mix = mix_.next();
            const float capture_gate = capture_gate_.next();
            for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                const float frozen = finite(overlap_[channel][overlap_cursor_]);
                overlap_[channel][overlap_cursor_] = 0.0F;
                const float wet = has_capture_
                                      ? dry[channel] + capture_gate * (frozen - dry[channel])
                                      : dry[channel];
                if (effect_mix == 0.0F || capture_gate == 0.0F || !has_capture_) {
                    samples[base + channel] = dry[channel];
                } else {
                    samples[base + channel] = std::clamp(
                        finite(dry[channel] + effect_mix * (wet - dry[channel])),
                        -kMaximumProcessedSample,
                        kMaximumProcessedSample
                    );
                }
            }
            overlap_cursor_ = (overlap_cursor_ + 1) % kOverlapCapacityFrames;
        }
    }

    void reset() noexcept {
        clear_channels(history_);
        clear_channels(dry_delay_);
        clear_channels(overlap_);
        clear_channels(analysis_input_);
        clear_channels(inverse_output_);
        clear_channels(spectrum_real_);
        clear_channels(spectrum_imaginary_);
        clear_channels(previous_phase_);
        clear_channels(captured_magnitude_);
        clear_channels(synthesis_phase_);
        clear_channels(phase_advance_);
        history_cursor_ = 0;
        dry_cursor_ = 0;
        overlap_cursor_ = 0;
        input_frames_ = 0;
        previous_analysis_frame_ = 0;
        next_synthesis_input_frame_ = 0;
        activation_countdown_ = 0;
        has_previous_analysis_ = false;
        capture_requested_ = false;
        has_capture_ = false;
        mix_.reset(target_mix(authored_));
        capture_gate_.reset(0.0F);
    }

    [[nodiscard]] bool has_capture() const noexcept {
        return has_capture_;
    }

    [[nodiscard]] bool capture_pending() const noexcept {
        return capture_requested_;
    }

    [[nodiscard]] FreezeVfxAdjustment adjustment() const noexcept {
        return authored_;
    }

  private:
    using ChannelVectors = std::array<std::vector<float>, kMaximumChannels>;

    [[nodiscard]] static ChannelVectors make_channel_vectors(std::size_t size) {
        return {std::vector<float>(size, 0.0F), std::vector<float>(size, 0.0F)};
    }

    static void clear_channels(ChannelVectors& channels) noexcept {
        for (auto& channel : channels) {
            std::fill(channel.begin(), channel.end(), 0.0F);
        }
    }

    [[nodiscard]] static float target_mix(FreezeVfxAdjustment adjustment) noexcept {
        return adjustment.enabled ? static_cast<float>(adjustment.mix_percent) / 100.0F : 0.0F;
    }

    void analyze(bool capture) noexcept {
        const std::uint64_t analysis_delta = has_previous_analysis_
                                                 ? input_frames_ - previous_analysis_frame_
                                                 : static_cast<std::uint64_t>(kHopFrames);
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            for (std::size_t frame = 0; frame < kWindowFrames; ++frame) {
                const std::size_t source = (history_cursor_ + frame) % kWindowFrames;
                analysis_input_[channel][frame] = history_[channel][source] * window_[frame];
            }
            fft_[channel]->fft(
                analysis_input_[channel].data(),
                spectrum_real_[channel].data(),
                spectrum_imaginary_[channel].data()
            );
            for (std::size_t bin = 0; bin < kBinCount; ++bin) {
                const float phase =
                    std::atan2(spectrum_imaginary_[channel][bin], spectrum_real_[channel][bin]);
                if (capture) {
                    captured_magnitude_[channel][bin] =
                        std::hypot(spectrum_real_[channel][bin], spectrum_imaginary_[channel][bin]);
                    synthesis_phase_[channel][bin] = phase;
                    if (bin == 0 || bin + 1 == kBinCount) {
                        phase_advance_[channel][bin] = 0.0F;
                    } else {
                        const float angular_frequency =
                            kTwoPi * static_cast<float>(bin) / static_cast<float>(kWindowFrames);
                        const float expected =
                            angular_frequency * static_cast<float>(analysis_delta);
                        const float residual =
                            has_previous_analysis_
                                ? std::remainder(
                                      phase - previous_phase_[channel][bin] - expected,
                                      kTwoPi
                                  )
                                : 0.0F;
                        const float scale =
                            static_cast<float>(kHopFrames) / static_cast<float>(analysis_delta);
                        phase_advance_[channel][bin] =
                            angular_frequency * static_cast<float>(kHopFrames) + residual * scale;
                    }
                }
                previous_phase_[channel][bin] = phase;
            }
        }
        previous_analysis_frame_ = input_frames_;
        has_previous_analysis_ = true;
    }

    void schedule_synthesis() noexcept {
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            for (std::size_t bin = 0; bin < kBinCount; ++bin) {
                spectrum_real_[channel][bin] =
                    captured_magnitude_[channel][bin] * std::cos(synthesis_phase_[channel][bin]);
                spectrum_imaginary_[channel][bin] =
                    captured_magnitude_[channel][bin] * std::sin(synthesis_phase_[channel][bin]);
            }
            spectrum_imaginary_[channel][0] = 0.0F;
            spectrum_imaginary_[channel][kBinCount - 1] = 0.0F;
            fft_[channel]->ifft(
                inverse_output_[channel].data(),
                spectrum_real_[channel].data(),
                spectrum_imaginary_[channel].data()
            );
            for (std::size_t frame = 0; frame < kWindowFrames; ++frame) {
                const std::size_t destination =
                    (overlap_cursor_ + kWindowFrames + frame) % kOverlapCapacityFrames;
                // Four 75%-overlapped periodic Hann products sum to two.
                overlap_[channel][destination] +=
                    finite(inverse_output_[channel][frame]) * window_[frame] * 0.5F;
            }
            for (std::size_t bin = 0; bin < kBinCount; ++bin) {
                synthesis_phase_[channel][bin] = std::remainder(
                    synthesis_phase_[channel][bin] + phase_advance_[channel][bin],
                    kTwoPi
                );
            }
        }
    }

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    FreezeVfxAdjustment authored_;
    std::size_t smoothing_frames_ = 1;
    ChannelVectors history_;
    ChannelVectors dry_delay_;
    ChannelVectors overlap_;
    ChannelVectors analysis_input_;
    ChannelVectors inverse_output_;
    ChannelVectors spectrum_real_;
    ChannelVectors spectrum_imaginary_;
    ChannelVectors previous_phase_;
    ChannelVectors captured_magnitude_;
    ChannelVectors synthesis_phase_;
    ChannelVectors phase_advance_;
    std::array<std::unique_ptr<audiofft::AudioFFT>, kMaximumChannels> fft_;
    std::vector<float> window_;
    std::size_t history_cursor_ = 0;
    std::size_t dry_cursor_ = 0;
    std::size_t overlap_cursor_ = 0;
    std::uint64_t input_frames_ = 0;
    std::uint64_t previous_analysis_frame_ = 0;
    std::uint64_t next_synthesis_input_frame_ = 0;
    std::size_t activation_countdown_ = 0;
    LinearRamp mix_;
    LinearRamp capture_gate_;
    bool has_previous_analysis_ = false;
    bool capture_requested_ = false;
    bool has_capture_ = false;
};

FreezeVfxProcessor::FreezeVfxProcessor(
    FreezeVfxAdjustment adjustment,
    std::uint32_t sample_rate,
    std::size_t channel_count
) : impl_(std::make_unique<Impl>(adjustment, sample_rate, channel_count)) {}

FreezeVfxProcessor::~FreezeVfxProcessor() = default;

void FreezeVfxProcessor::update(FreezeVfxAdjustment adjustment) {
    impl_->update(adjustment);
}

bool FreezeVfxProcessor::request_capture() noexcept {
    return impl_->request_capture();
}

void FreezeVfxProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count
) {
    impl_->process_interleaved(samples, frame_count, channel_count);
}

void FreezeVfxProcessor::reset() noexcept {
    impl_->reset();
}

bool FreezeVfxProcessor::has_capture() const noexcept {
    return impl_->has_capture();
}

bool FreezeVfxProcessor::capture_pending() const noexcept {
    return impl_->capture_pending();
}

FreezeVfxAdjustment FreezeVfxProcessor::adjustment() const noexcept {
    return impl_->adjustment();
}

} // namespace echo::audio
