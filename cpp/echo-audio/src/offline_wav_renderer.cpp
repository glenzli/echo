#include "echo/audio/offline_wav_renderer.hpp"

#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <chrono>
#include <cmath>
#include <cstring>
#include <limits>
#include <thread>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kPcmFormat = 1;
constexpr std::uint32_t kSampleRate = 48000;
constexpr std::uint16_t kChannels = 2;
constexpr std::uint16_t kBitDepth = 24;
constexpr std::uint16_t kBytesPerSample = 3;
constexpr std::size_t kChunkFrames = 4096;
constexpr std::uint64_t kWavHeaderBytes = 44;

template <typename Integer>
void put_little_endian(std::span<std::byte> destination, std::size_t offset, Integer value) {
    static_assert(std::is_unsigned_v<Integer>);
    for (std::size_t index = 0; index < sizeof(Integer); ++index) {
        destination[offset + index] =
            std::byte{static_cast<unsigned char>((value >> (index * 8U)) & 0xffU)};
    }
}

void put_fourcc(std::span<std::byte> destination, std::size_t offset, const char* value) {
    std::memcpy(destination.data() + offset, value, 4);
}

std::array<std::byte, kWavHeaderBytes> wav_header(std::uint32_t data_size) {
    std::array<std::byte, kWavHeaderBytes> header{};
    put_fourcc(header, 0, "RIFF");
    put_little_endian<std::uint32_t>(header, 4, 36U + data_size);
    put_fourcc(header, 8, "WAVE");
    put_fourcc(header, 12, "fmt ");
    put_little_endian<std::uint32_t>(header, 16, 16U);
    put_little_endian<std::uint16_t>(header, 20, static_cast<std::uint16_t>(kPcmFormat));
    put_little_endian<std::uint16_t>(header, 22, kChannels);
    put_little_endian<std::uint32_t>(header, 24, kSampleRate);
    put_little_endian<std::uint32_t>(header, 28, kSampleRate * kChannels * kBytesPerSample);
    put_little_endian<std::uint16_t>(header, 32, kChannels * kBytesPerSample);
    put_little_endian<std::uint16_t>(header, 34, kBitDepth);
    put_fourcc(header, 36, "data");
    put_little_endian<std::uint32_t>(header, 40, data_size);
    return header;
}

class Dither {
  public:
    float tpdf() {
        return (uniform() - uniform()) / 8388608.0F;
    }

  private:
    float uniform() {
        state_ ^= state_ << 13U;
        state_ ^= state_ >> 17U;
        state_ ^= state_ << 5U;
        return static_cast<float>(state_ & 0x00ff'ffffU) / 16777216.0F;
    }

    std::uint32_t state_ = 0x4543'484fU;
};

std::int32_t quantize(float sample, Dither& dither) {
    const float prepared = std::clamp(sample + dither.tpdf(), -1.0F, 0.99999988F);
    return std::clamp(
        static_cast<std::int32_t>(std::lrint(prepared * 8388607.0F)),
        -8388608,
        8388607
    );
}

void append_pcm24(
    std::span<const float> samples,
    std::vector<std::byte>& encoded,
    std::vector<float>& measured,
    Dither& dither
) {
    encoded.resize(samples.size() * kBytesPerSample);
    measured.resize(samples.size());
    for (std::size_t index = 0; index < samples.size(); ++index) {
        const std::int32_t value = quantize(samples[index], dither);
        const std::uint32_t bits = static_cast<std::uint32_t>(value);
        encoded[index * 3] = std::byte{static_cast<unsigned char>(bits & 0xffU)};
        encoded[index * 3 + 1] = std::byte{static_cast<unsigned char>((bits >> 8U) & 0xffU)};
        encoded[index * 3 + 2] = std::byte{static_cast<unsigned char>((bits >> 16U) & 0xffU)};
        measured[index] = static_cast<float>(value) / 8388607.0F;
    }
}

bool cancelled(const OfflineRenderCallbacks& callbacks) {
    return callbacks.cancelled && callbacks.cancelled();
}

void report_progress(const OfflineRenderCallbacks& callbacks, double value) {
    if (callbacks.progress) {
        callbacks.progress(std::clamp(value, 0.0, 1.0));
    }
}

} // namespace

OfflineRenderCancelled::OfflineRenderCancelled() : std::runtime_error("offline render cancelled") {}

OfflineWavRenderResult OfflineWavRenderer::render(
    const std::string& sourcePath,
    const PlaybackAdjustment& adjustment,
    RenderByteSink& sink,
    const OfflineRenderCallbacks& callbacks
) {
    if (adjustment.trim_end_millis <= adjustment.trim_start_millis) {
        throw std::invalid_argument("offline render requires a non-empty selection");
    }
    const std::uint64_t selected_millis = adjustment.trim_end_millis - adjustment.trim_start_millis;
    if (selected_millis > std::numeric_limits<std::uint64_t>::max() / kSampleRate) {
        throw std::invalid_argument("selection exceeds the WAV v1 size limit");
    }
    const std::uint64_t expected_frames = selected_millis * kSampleRate / 1000U;
    constexpr std::uint64_t kBytesPerFrame = kChannels * kBytesPerSample;
    if (expected_frames > std::numeric_limits<std::uint64_t>::max() / kBytesPerFrame) {
        throw std::invalid_argument("selection exceeds the WAV v1 size limit");
    }
    const std::uint64_t expected_data_bytes = expected_frames * kBytesPerFrame;
    if (expected_data_bytes > std::numeric_limits<std::uint32_t>::max() - 36U) {
        throw std::invalid_argument("selection exceeds the WAV v1 size limit");
    }
    const auto placeholder = wav_header(0);
    sink.write(placeholder);

    PlaybackSession session(
        sourcePath,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    OfflineLoudnessAnalyzer analyzer(session.sample_rate(), session.channel_count());
    std::vector<float> decoded(kChunkFrames * session.channel_count());
    std::vector<float> measured;
    std::vector<std::byte> encoded;
    Dither dither;
    std::uint64_t frame_count = 0;
    std::uint64_t data_bytes = 0;

    while (true) {
        if (cancelled(callbacks)) {
            session.stop();
            throw OfflineRenderCancelled();
        }
        const std::size_t frames = session.read(decoded.data(), kChunkFrames);
        if (frames == 0) {
            if (session.is_ended() && session.buffered_frames() == 0) {
                break;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            continue;
        }
        const std::size_t sample_count = frames * session.channel_count();
        append_pcm24(
            std::span<const float>(decoded.data(), sample_count),
            encoded,
            measured,
            dither
        );
        sink.write(encoded);
        analyzer.process_interleaved(measured.data(), frames, session.channel_count());
        frame_count += frames;
        data_bytes += encoded.size();
        report_progress(
            callbacks,
            expected_frames == 0
                ? 0.0
                : static_cast<double>(frame_count) / static_cast<double>(expected_frames)
        );
    }
    session.stop();
    if (data_bytes > std::numeric_limits<std::uint32_t>::max() - 36U) {
        throw std::runtime_error("rendered data exceeds the WAV v1 size limit");
    }
    sink.seek(0);
    const auto header = wav_header(static_cast<std::uint32_t>(data_bytes));
    sink.write(header);
    report_progress(callbacks, 1.0);
    const OfflineLoudnessResult loudness = analyzer.result();
    return {
        .frame_count = frame_count,
        .size_bytes = kWavHeaderBytes + data_bytes,
        .sample_rate = session.sample_rate(),
        .channel_count = session.channel_count(),
        .bit_depth = kBitDepth,
        .integrated_lufs = loudness.integrated_lufs,
        .true_peak_dbtp = loudness.true_peak_dbtp,
    };
}

} // namespace echo::audio
