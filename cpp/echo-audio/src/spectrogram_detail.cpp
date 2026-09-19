#include "echo/audio/spectrogram_detail.hpp"

#include "convolution/signalsmith_audiofft_adapter.hpp"
#include "echo/audio/decode.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <numbers>
#include <stdexcept>
#include <thread>

namespace echo::audio {

SpectrogramDetail build_spectrogram_detail(
    const std::string& path,
    const SpectrogramDetailRequest& request,
    std::stop_token cancellation
) {
    if (request.start_millis >= request.end_millis || request.end_millis > 14'400'000
        || !std::isfinite(request.low_hertz) || !std::isfinite(request.high_hertz)
        || request.low_hertz < 20 || request.high_hertz > 24'000
        || request.low_hertz >= request.high_hertz
        || (request.window_frames != 2'048 && request.window_frames != 8'192)
        || request.columns == 0 || request.columns > 1'024 || request.rows == 0
        || request.rows > 512 || !std::isfinite(request.floor_decibels)
        || !std::isfinite(request.ceiling_decibels) || request.floor_decibels < -144
        || request.ceiling_decibels > 12
        || request.ceiling_decibels - request.floor_decibels < 12) {
        throw std::invalid_argument("invalid spectrogram viewport");
    }
    const auto probe_result = probe(path);
    if (!probe_result.has_audio || request.start_millis >= probe_result.duration_millis) {
        throw std::invalid_argument("spectrogram viewport is outside the source");
    }
    constexpr std::uint64_t frames_per_millisecond = 48;
    const std::size_t window_size = request.window_frames;
    const std::uint64_t half = window_size / 2;
    const std::uint64_t start = request.start_millis * frames_per_millisecond;
    const std::uint64_t end =
        std::min(request.end_millis, probe_result.duration_millis) * frames_per_millisecond;
    PlaybackAdjustment adjustment;
    adjustment.trim_start_millis = start > half ? (start - half) / frames_per_millisecond : 0;
    adjustment.trim_end_millis = std::min(
        probe_result.duration_millis,
        request.end_millis + (half + frames_per_millisecond - 1) / frames_per_millisecond
    );
    PlaybackSession session(
        path,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    const std::size_t channels = session.channel_count();
    const std::size_t bins = window_size / 2 + 1;
    const std::uint64_t hop = std::max<std::uint64_t>(
        1,
        std::min<std::uint64_t>(
            window_size / 4,
            (end - start + request.columns - 1) / request.columns
        )
    );
    SpectrogramDetail result{
        request.columns,
        request.rows,
        std::vector<std::uint8_t>(static_cast<std::size_t>(request.columns) * request.rows, 0)
    };
    audiofft::AudioFFT fft;
    fft.init(window_size);
    std::vector<float> history(window_size * channels, 0);
    std::vector<float> window(window_size), frame(window_size), real(bins), imaginary(bins);
    std::vector<float> magnitudes(bins), buffer(4'096 * channels);
    std::vector<std::pair<std::size_t, std::size_t>> row_bins;
    for (std::size_t index = 0; index < window_size; ++index) {
        window[index] = 0.5F
                        - 0.5F
                              * std::cos(
                                  2.0F * std::numbers::pi_v<float>
                                  * static_cast<float>(index) / static_cast<float>(window_size)
                              );
    }
    const auto frequency = [&](double ratio) {
        return request.logarithmic
                   ? request.low_hertz * std::pow(request.high_hertz / request.low_hertz, ratio)
                   : request.low_hertz + (request.high_hertz - request.low_hertz) * ratio;
    };
    for (std::uint32_t row = 0; row < request.rows; ++row) {
        const double low =
            frequency(static_cast<double>(row) / request.rows) * window_size / 48'000;
        const double high =
            frequency(static_cast<double>(row + 1) / request.rows) * window_size / 48'000;
        const auto first = std::min(bins - 1, static_cast<std::size_t>(std::floor(low)));
        const auto last =
            std::min(bins, std::max(first + 1, static_cast<std::size_t>(std::ceil(high))));
        row_bins.emplace_back(first, last);
    }
    std::size_t cursor = 0;
    std::uint64_t source_frame = adjustment.trim_start_millis * frames_per_millisecond;
    std::uint64_t center = start;
    const auto emit = [&] {
        std::fill(magnitudes.begin(), magnitudes.end(), 0);
        for (std::size_t channel = 0; channel < channels; ++channel) {
            for (std::size_t index = 0; index < window_size; ++index) {
                frame[index] =
                    history[((cursor + index) % window_size) * channels + channel] * window[index];
            }
            fft.fft(frame.data(), real.data(), imaginary.data());
            for (std::size_t bin = 0; bin < bins; ++bin) {
                // Hann coherent-gain correction for a full-scale bin-centered sine.
                const float magnitude =
                    std::hypot(real[bin], imaginary[bin]) * 4.0F / static_cast<float>(window_size);
                magnitudes[bin] = std::max(magnitudes[bin], magnitude);
            }
        }
        const auto column = std::min<std::uint64_t>(
            request.columns - 1,
            (center - start) * request.columns / (end - start)
        );
        const auto next_column = std::min<std::uint64_t>(
            request.columns,
            ((std::min(center + hop, end) - start) * request.columns + end - start - 1)
                / (end - start)
        );
        for (std::uint32_t row = 0; row < request.rows; ++row) {
            float maximum = 0;
            for (auto bin = row_bins[row].first; bin < row_bins[row].second; ++bin) {
                maximum = std::max(maximum, magnitudes[bin]);
            }
            const float db = 20.0F * std::log10(std::max(maximum, 1.0E-9F));
            const auto value = static_cast<std::uint8_t>(std::lround(
                255.0F
                * std::clamp(
                    (db - request.floor_decibels)
                        / (request.ceiling_decibels - request.floor_decibels),
                    0.0F,
                    1.0F
                )
            ));
            for (auto x = column; x < std::max(column + 1, next_column); ++x) {
                auto& target = result.magnitudes[x * request.rows + row];
                target = std::max(target, value);
            }
        }
    };
    const auto consume = [&](const float* samples) {
        for (std::size_t channel = 0; channel < channels; ++channel) {
            const float sample = samples == nullptr ? 0 : samples[channel];
            history[cursor * channels + channel] = std::isfinite(sample) ? sample : 0;
        }
        cursor = (cursor + 1) % window_size;
        ++source_frame;
        if (center < end && source_frame >= center + half) {
            emit();
            center += hop;
        }
    };
    while (!cancellation.stop_requested() && center < end) {
        const auto count = session.read(buffer.data(), 4'096);
        for (std::size_t index = 0; index < count && center < end; ++index) {
            consume(buffer.data() + index * channels);
        }
        if (count == 0) {
            if (session.is_ended() || session.is_stopped())
                break;
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    }
    while (!cancellation.stop_requested() && center < end)
        consume(nullptr);
    if (cancellation.stop_requested())
        throw std::runtime_error("spectrogram cancelled");
    return result;
}

} // namespace echo::audio
