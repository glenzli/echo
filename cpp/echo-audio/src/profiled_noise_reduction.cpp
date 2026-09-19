#include "echo/audio/profiled_noise_reduction.hpp"

#include "convolution/signalsmith_audiofft_adapter.hpp"
#include "echo/audio/decode.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <numbers>
#include <stdexcept>
#include <thread>
#include <utility>

namespace echo::audio {

void validate_noise_profile(const ProfiledNoiseReduction& settings, std::uint64_t duration_millis) {
    if (settings.algorithm_version != 1
        || settings.capture_start_millis >= settings.capture_end_millis
        || settings.capture_end_millis > duration_millis
        || settings.capture_end_millis - settings.capture_start_millis < 100
        || settings.capture_end_millis - settings.capture_start_millis > 30'000
        || settings.power_centibels.size() != kNoiseProfileBins || settings.reduction_centibels < 0
        || settings.reduction_centibels > 3'600 || settings.sensitivity_centibels < 0
        || settings.sensitivity_centibels > 1'200 || settings.smoothing_bins > 8
        || std::any_of(
            settings.power_centibels.begin(),
            settings.power_centibels.end(),
            [](auto value) { return value < -14'400 || value > 1'200; }
        )) {
        throw std::invalid_argument("invalid learned noise profile");
    }
}

ProfiledNoiseReduction learn_noise_profile(
    const std::string& path,
    std::uint64_t start_millis,
    std::uint64_t end_millis,
    std::stop_token cancellation
) {
    const auto source = probe(path);
    if (!source.has_audio || end_millis > source.duration_millis || start_millis >= end_millis
        || end_millis - start_millis < 100 || end_millis - start_millis > 30'000)
        throw std::invalid_argument("noise sample must contain 0.1 to 30 seconds of source audio");
    PlaybackAdjustment adjustment;
    adjustment.trim_start_millis = start_millis;
    adjustment.trim_end_millis = end_millis;
    PlaybackSession session(
        path,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    const std::size_t channels = session.channel_count();
    const auto maximum_frames = static_cast<std::size_t>((end_millis - start_millis) * 48);
    std::vector<float> samples;
    samples.reserve(maximum_frames * channels);
    std::vector<float> buffer(4'096 * channels);
    while (!cancellation.stop_requested() && samples.size() / channels < maximum_frames) {
        const auto count = session.read(
            buffer.data(),
            std::min<std::size_t>(4'096, maximum_frames - samples.size() / channels)
        );
        samples.insert(
            samples.end(),
            buffer.begin(),
            buffer.begin() + static_cast<std::ptrdiff_t>(count * channels)
        );
        if (count == 0) {
            if (session.is_ended() || session.is_stopped())
                break;
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    }
    audiofft::AudioFFT fft;
    fft.init(kNoiseProfileWindowFrames);
    std::vector<float> window(kNoiseProfileWindowFrames), frame(kNoiseProfileWindowFrames);
    std::vector<float> real(kNoiseProfileBins), imaginary(kNoiseProfileBins),
        maximum(kNoiseProfileBins);
    std::vector<double> sum(kNoiseProfileBins, 0);
    for (std::size_t index = 0; index < window.size(); ++index)
        window[index] = 0.5F
                        - 0.5F
                              * std::cos(
                                  2.0F * std::numbers::pi_v<float>
                                  * static_cast<float>(index) / static_cast<float>(window.size())
                              );
    constexpr float scale = 4.0F / static_cast<float>(kNoiseProfileWindowFrames);
    std::size_t windows = 0;
    for (std::size_t start = 0; start + kNoiseProfileWindowFrames <= samples.size() / channels;
         start += kNoiseProfileWindowFrames / 4) {
        if (cancellation.stop_requested())
            break;
        std::fill(maximum.begin(), maximum.end(), 0);
        for (std::size_t channel = 0; channel < channels; ++channel) {
            for (std::size_t index = 0; index < frame.size(); ++index) {
                const float sample = samples[(start + index) * channels + channel];
                frame[index] = std::isfinite(sample) ? sample * window[index] : 0;
            }
            fft.fft(frame.data(), real.data(), imaginary.data());
            for (std::size_t bin = 0; bin < maximum.size(); ++bin)
                maximum[bin] = std::max(
                    maximum[bin],
                    (real[bin] * real[bin] + imaginary[bin] * imaginary[bin]) * scale * scale
                );
        }
        for (std::size_t bin = 0; bin < sum.size(); ++bin)
            sum[bin] += maximum[bin];
        ++windows;
    }
    if (cancellation.stop_requested())
        throw std::runtime_error("noise learning cancelled");
    if (windows == 0
        || *std::max_element(sum.begin(), sum.end()) / static_cast<double>(windows) < 1.0E-14)
        throw std::invalid_argument("noise sample is silent or too short");
    ProfiledNoiseReduction result;
    result.capture_start_millis = start_millis;
    result.capture_end_millis = end_millis;
    for (const auto power : sum)
        result.power_centibels.push_back(
            static_cast<std::int16_t>(std::lround(
                std::clamp(
                    1'000.0 * std::log10(std::max(power / static_cast<double>(windows), 1.0E-15)),
                    -14'400.0,
                    1'200.0
                )
            ))
        );
    validate_noise_profile(result, source.duration_millis);
    return result;
}

ProfiledNoiseReducer::ProfiledNoiseReducer(ProfiledNoiseReduction settings) :
    settings_(std::move(settings)) {
    validate_noise_profile(settings_, settings_.capture_end_millis);
    for (auto value : settings_.power_centibels)
        noise_power_.push_back(std::pow(10.0F, static_cast<float>(value) / 1'000.0F));
    previous_power_.resize(kNoiseProfileBins);
    previous_gain_.resize(kNoiseProfileBins);
}

void ProfiledNoiseReducer::reset() {
    started_ = false;
}

std::vector<float> ProfiledNoiseReducer::gains(std::span<const float> powers) {
    if (powers.size() != kNoiseProfileBins)
        throw std::invalid_argument("noise spectrum size mismatch");
    const float floor =
        std::pow(10.0F, -static_cast<float>(settings_.reduction_centibels) / 2'000.0F);
    const float sensitivity =
        std::pow(10.0F, static_cast<float>(settings_.sensitivity_centibels) / 1'000.0F);
    std::vector<float> target(kNoiseProfileBins), result(kNoiseProfileBins);
    for (std::size_t bin = 0; bin < target.size(); ++bin) {
        const float power = std::isfinite(powers[bin]) ? std::max(0.0F, powers[bin]) : 0;
        previous_power_[bin] = started_ ? 0.84F * previous_power_[bin] + 0.16F * power : power;
        // Immediate signal attacks remain audible; the averaged floor controls the decay.
        const float denominator = std::max({power, previous_power_[bin], 1.0E-15F});
        target[bin] = std::max(
            floor,
            std::sqrt(std::max(0.0F, 1.0F - sensitivity * noise_power_[bin] / denominator))
        );
    }
    for (std::size_t bin = 0; bin < target.size(); ++bin) {
        const auto first = bin > settings_.smoothing_bins ? bin - settings_.smoothing_bins : 0;
        const auto last = std::min(target.size(), bin + settings_.smoothing_bins + 1);
        // Preserve neighbouring signal bins instead of smearing attenuation into them.
        float gain = *std::max_element(
            target.begin() + static_cast<std::ptrdiff_t>(first),
            target.begin() + static_cast<std::ptrdiff_t>(last)
        );
        if (started_) {
            const float amount = gain > previous_gain_[bin] ? 0.85F : 0.125F;
            gain = previous_gain_[bin] + amount * (gain - previous_gain_[bin]);
        }
        previous_gain_[bin] = gain;
        result[bin] = settings_.residue ? 1.0F - gain : gain;
    }
    started_ = true;
    return result;
}

} // namespace echo::audio
