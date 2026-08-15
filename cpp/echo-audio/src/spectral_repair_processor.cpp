#include "echo/audio/spectral_repair_processor.hpp"

#include "convolution/signalsmith_audiofft_adapter.hpp"

#include <algorithm>
#include <cmath>
#include <limits>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::size_t kSpectrumBins = SpectralRepairProcessor::kWindowFrames / 2 + 1;
constexpr std::size_t kMaximumRegions = 64;

float feather_weight(float value, float low, float high, float feather) {
    if (value < low || value > high) {
        return 0.0F;
    }
    if (feather <= 0.0F) {
        return 1.0F;
    }
    return std::clamp(std::min((value - low) / feather, (high - value) / feather), 0.0F, 1.0F);
}

float bin_gain(
    float source_millis,
    float hertz,
    const std::vector<SpectralAttenuationRegion>& regions
) {
    float attenuation_decibels = 0.0F;
    for (const auto& region : regions) {
        if (region.attenuation_centibels == 0) {
            continue;
        }
        const float time = feather_weight(
            source_millis,
            static_cast<float>(region.start_millis),
            static_cast<float>(region.end_millis),
            static_cast<float>(region.time_feather_millis)
        );
        const float frequency = feather_weight(
            hertz,
            static_cast<float>(region.low_hertz),
            static_cast<float>(region.high_hertz),
            static_cast<float>(region.frequency_feather_hertz)
        );
        attenuation_decibels +=
            static_cast<float>(region.attenuation_centibels) / 100.0F * time * frequency;
    }
    return std::pow(10.0F, -std::min(96.0F, attenuation_decibels) / 20.0F);
}

} // namespace

void SpectralRepairProcessor::validate_regions(
    const std::vector<SpectralAttenuationRegion>& regions,
    std::uint64_t source_duration_millis,
    std::uint32_t sample_rate
) {
    if (sample_rate == 0 || regions.size() > kMaximumRegions) {
        throw std::invalid_argument("spectral repair configuration is invalid");
    }
    const auto nyquist = static_cast<std::uint32_t>(sample_rate / 2);
    for (const auto& region : regions) {
        if (region.start_millis >= region.end_millis || region.end_millis > source_duration_millis
            || region.low_hertz < 20 || region.low_hertz >= region.high_hertz
            || region.high_hertz > nyquist || region.attenuation_centibels < 0
            || region.attenuation_centibels > 9'600 || region.time_feather_millis > 250
            || region.frequency_feather_hertz > 2'000) {
            throw std::invalid_argument("spectral repair region is invalid");
        }
    }
}

void SpectralRepairProcessor::process_interleaved(
    float* samples,
    std::size_t frame_count,
    std::size_t channel_count,
    std::uint32_t sample_rate,
    std::uint64_t first_source_frame,
    const std::vector<SpectralAttenuationRegion>& regions
) {
    if (samples == nullptr || frame_count == 0 || channel_count == 0 || regions.empty()) {
        return;
    }
    const auto duration_millis = static_cast<std::uint64_t>(
        std::ceil(static_cast<double>(first_source_frame + frame_count) * 1000.0 / sample_rate)
    );
    validate_regions(regions, duration_millis, sample_rate);

    audiofft::AudioFFT fft;
    fft.init(kWindowFrames);
    std::vector<float> window(kWindowFrames);
    std::vector<float> real(kSpectrumBins);
    std::vector<float> imaginary(kSpectrumBins);
    std::vector<float> output(frame_count, 0.0F);
    std::vector<float> normalization(frame_count, 0.0F);
    for (std::size_t frame = 0; frame < kWindowFrames; ++frame) {
        const float phase = 2.0F * std::numbers::pi_v<float>
                            * static_cast<float>(frame) / static_cast<float>(kWindowFrames);
        window[frame] = 0.5F - 0.5F * std::cos(phase);
    }
    for (std::size_t channel = 0; channel < channel_count; ++channel) {
        std::fill(output.begin(), output.end(), 0.0F);
        std::fill(normalization.begin(), normalization.end(), 0.0F);
        for (std::size_t start = 0; start < frame_count; start += kHopFrames) {
            std::vector<float> frame(kWindowFrames, 0.0F);
            for (std::size_t index = 0; index < kWindowFrames && start + index < frame_count;
                 ++index) {
                frame[index] = samples[(start + index) * channel_count + channel] * window[index];
            }
            fft.fft(frame.data(), real.data(), imaginary.data());
            const float source_millis =
                static_cast<float>(first_source_frame + start + kWindowFrames / 2) * 1000.0F
                / static_cast<float>(sample_rate);
            for (std::size_t bin = 0; bin < kSpectrumBins; ++bin) {
                const float gain = bin_gain(
                    source_millis,
                    static_cast<float>(bin) * static_cast<float>(sample_rate)
                        / static_cast<float>(kWindowFrames),
                    regions
                );
                real[bin] *= gain;
                imaginary[bin] *= gain;
            }
            fft.ifft(frame.data(), real.data(), imaginary.data());
            for (std::size_t index = 0; index < kWindowFrames && start + index < frame_count;
                 ++index) {
                output[start + index] += frame[index] * window[index];
                normalization[start + index] += window[index] * window[index];
            }
        }
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            samples[frame * channel_count + channel] =
                normalization[frame] > 1.0E-6F ? output[frame] / normalization[frame] : 0.0F;
        }
    }
}

SpectralRepairStream::SpectralRepairStream(
    std::uint32_t sample_rate,
    std::size_t channel_count,
    std::vector<SpectralAttenuationRegion> regions
) : sample_rate_(sample_rate), channel_count_(channel_count), regions_(std::move(regions)) {
    if (sample_rate_ == 0 || channel_count_ == 0) {
        throw std::invalid_argument("spectral repair stream format is invalid");
    }
    window_.resize(SpectralRepairProcessor::kWindowFrames);
    for (std::size_t frame = 0; frame < window_.size(); ++frame) {
        const float phase = 2.0F * std::numbers::pi_v<float>
                            * static_cast<float>(frame) / static_cast<float>(window_.size());
        window_[frame] = 0.5F - 0.5F * std::cos(phase);
    }
    input_.resize(SpectralRepairProcessor::kWindowFrames * channel_count_);
    overlap_add_.assign(SpectralRepairProcessor::kWindowFrames * channel_count_, 0.0F);
    normalization_.assign(SpectralRepairProcessor::kWindowFrames, 0.0F);
}

void SpectralRepairStream::reset() {
    input_frames_ = 0;
    std::fill(overlap_add_.begin(), overlap_add_.end(), 0.0F);
    std::fill(normalization_.begin(), normalization_.end(), 0.0F);
    ready_.clear();
    ready_offset_frames_ = 0;
    ready_first_source_frame_ = 0;
    next_input_source_frame_ = 0;
    next_output_source_frame_ = 0;
    received_end_source_frame_ = 0;
    started_ = false;
    finished_ = false;
}

void SpectralRepairStream::push_interleaved(
    const float* samples,
    std::size_t frame_count,
    std::uint64_t first_source_frame
) {
    if (samples == nullptr || frame_count == 0) {
        return;
    }
    if (finished_) {
        throw std::logic_error("cannot append to a finished spectral repair stream");
    }
    if (!started_) {
        started_ = true;
        next_input_source_frame_ = first_source_frame;
        next_output_source_frame_ = first_source_frame;
        received_end_source_frame_ = first_source_frame;
    } else if (first_source_frame != next_input_source_frame_) {
        throw std::invalid_argument("spectral repair input is not source-contiguous");
    }
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        std::copy_n(
            samples + frame * channel_count_,
            channel_count_,
            input_.data() + input_frames_ * channel_count_
        );
        ++input_frames_;
        ++next_input_source_frame_;
        ++received_end_source_frame_;
        if (input_frames_ == SpectralRepairProcessor::kWindowFrames) {
            process_window();
            emit_hop();
        }
    }
}

void SpectralRepairStream::append_zero_frame() {
    std::fill_n(input_.data() + input_frames_ * channel_count_, channel_count_, 0.0F);
    ++input_frames_;
    ++next_input_source_frame_;
}

void SpectralRepairStream::finish() {
    if (finished_) {
        return;
    }
    finished_ = true;
    if (!started_) {
        return;
    }
    while (next_output_source_frame_ < received_end_source_frame_) {
        while (input_frames_ < SpectralRepairProcessor::kWindowFrames) {
            append_zero_frame();
        }
        process_window();
        emit_hop();
    }
}

void SpectralRepairStream::process_window() {
    audiofft::AudioFFT fft;
    fft.init(SpectralRepairProcessor::kWindowFrames);
    std::vector<float> frame(SpectralRepairProcessor::kWindowFrames);
    std::vector<float> real(kSpectrumBins);
    std::vector<float> imaginary(kSpectrumBins);
    const float source_millis =
        static_cast<float>(next_output_source_frame_ + SpectralRepairProcessor::kWindowFrames / 2)
        * 1000.0F / static_cast<float>(sample_rate_);
    for (std::size_t channel = 0; channel < channel_count_; ++channel) {
        for (std::size_t index = 0; index < SpectralRepairProcessor::kWindowFrames; ++index) {
            frame[index] = input_[index * channel_count_ + channel] * window_[index];
        }
        fft.fft(frame.data(), real.data(), imaginary.data());
        for (std::size_t bin = 0; bin < kSpectrumBins; ++bin) {
            const float gain = bin_gain(
                source_millis,
                static_cast<float>(bin) * static_cast<float>(sample_rate_)
                    / static_cast<float>(SpectralRepairProcessor::kWindowFrames),
                regions_
            );
            real[bin] *= gain;
            imaginary[bin] *= gain;
        }
        fft.ifft(frame.data(), real.data(), imaginary.data());
        for (std::size_t index = 0; index < SpectralRepairProcessor::kWindowFrames; ++index) {
            overlap_add_[index * channel_count_ + channel] += frame[index] * window_[index];
        }
    }
    for (std::size_t index = 0; index < SpectralRepairProcessor::kWindowFrames; ++index) {
        normalization_[index] += window_[index] * window_[index];
    }
}

void SpectralRepairStream::emit_hop() {
    if (ready_.empty()) {
        ready_first_source_frame_ = next_output_source_frame_;
    }
    const std::size_t emitted_frames =
        finished_ ? static_cast<std::size_t>(std::min<std::uint64_t>(
                        SpectralRepairProcessor::kHopFrames,
                        received_end_source_frame_ - next_output_source_frame_
                    ))
                  : SpectralRepairProcessor::kHopFrames;
    const std::size_t ready_frames = ready_.size() / channel_count_;
    ready_.resize((ready_frames + emitted_frames) * channel_count_);
    for (std::size_t frame = 0; frame < emitted_frames; ++frame) {
        const float divisor = normalization_[frame];
        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
            ready_[(ready_frames + frame) * channel_count_ + channel] =
                divisor > 1.0E-6F ? overlap_add_[frame * channel_count_ + channel] / divisor : 0.0F;
        }
    }
    const std::size_t hop_samples = SpectralRepairProcessor::kHopFrames * channel_count_;
    std::move(
        input_.begin() + static_cast<std::ptrdiff_t>(hop_samples),
        input_.end(),
        input_.begin()
    );
    std::move(
        overlap_add_.begin() + static_cast<std::ptrdiff_t>(hop_samples),
        overlap_add_.end(),
        overlap_add_.begin()
    );
    std::move(
        normalization_.begin() + static_cast<std::ptrdiff_t>(SpectralRepairProcessor::kHopFrames),
        normalization_.end(),
        normalization_.begin()
    );
    std::fill(
        overlap_add_.end() - static_cast<std::ptrdiff_t>(hop_samples),
        overlap_add_.end(),
        0.0F
    );
    std::fill(
        normalization_.end() - static_cast<std::ptrdiff_t>(SpectralRepairProcessor::kHopFrames),
        normalization_.end(),
        0.0F
    );
    input_frames_ -= SpectralRepairProcessor::kHopFrames;
    next_output_source_frame_ += SpectralRepairProcessor::kHopFrames;
}

std::size_t SpectralRepairStream::drain_interleaved(
    float* output,
    std::uint64_t* source_frames,
    std::size_t capacity_frames
) {
    const std::size_t ready_frames = ready_.size() / channel_count_ - ready_offset_frames_;
    const std::size_t count = std::min(ready_frames, capacity_frames);
    if (count == 0 || output == nullptr) {
        return 0;
    }
    std::copy_n(
        ready_.data() + ready_offset_frames_ * channel_count_,
        count * channel_count_,
        output
    );
    if (source_frames != nullptr) {
        for (std::size_t frame = 0; frame < count; ++frame) {
            source_frames[frame] = ready_first_source_frame_ + ready_offset_frames_ + frame;
        }
    }
    ready_offset_frames_ += count;
    if (ready_offset_frames_ == ready_.size() / channel_count_) {
        ready_.clear();
        ready_offset_frames_ = 0;
    }
    return count;
}

} // namespace echo::audio
