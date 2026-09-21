#include "echo/audio/offline_wav_renderer.hpp"
#include "echo/audio/export_metadata.hpp"

#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <chrono>
#include <cmath>
#include <cstring>
#include <limits>
#include <string>
#include <thread>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kPcmFormat = 1;
constexpr std::uint32_t kSampleRate = 48000;
constexpr std::uint16_t kChannels = 2;
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

std::array<std::byte, kWavHeaderBytes>
wav_header(std::uint32_t data_size, std::uint16_t bit_depth, std::uint32_t metadata_size = 0) {
    const auto bytes_per_sample = static_cast<std::uint16_t>(bit_depth / 8U);
    std::array<std::byte, kWavHeaderBytes> header{};
    put_fourcc(header, 0, "RIFF");
    put_little_endian<std::uint32_t>(header, 4, 36U + data_size + metadata_size);
    put_fourcc(header, 8, "WAVE");
    put_fourcc(header, 12, "fmt ");
    put_little_endian<std::uint32_t>(header, 16, 16U);
    put_little_endian<std::uint16_t>(header, 20, static_cast<std::uint16_t>(kPcmFormat));
    put_little_endian<std::uint16_t>(header, 22, kChannels);
    put_little_endian<std::uint32_t>(header, 24, kSampleRate);
    put_little_endian<std::uint32_t>(header, 28, kSampleRate * kChannels * bytes_per_sample);
    put_little_endian<std::uint16_t>(header, 32, kChannels * bytes_per_sample);
    put_little_endian<std::uint16_t>(header, 34, bit_depth);
    put_fourcc(header, 36, "data");
    put_little_endian<std::uint32_t>(header, 40, data_size);
    return header;
}

class Dither {
  public:
    float tpdf(float quantization_scale) {
        return (uniform() - uniform()) / quantization_scale;
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

std::int32_t quantize(float sample, Dither& dither, std::int32_t scale) {
    const float quantization_scale = static_cast<float>(scale + 1);
    const float prepared = std::clamp(
        sample + dither.tpdf(quantization_scale),
        -1.0F,
        static_cast<float>(scale) / quantization_scale
    );
    return std::clamp(
        static_cast<std::int32_t>(std::lrint(prepared * static_cast<float>(scale))),
        -scale - 1,
        scale
    );
}

void append_pcm(
    std::span<const float> samples,
    std::vector<std::byte>& encoded,
    std::vector<float>& measured,
    Dither& dither,
    WavPcmDepth depth
) {
    const bool is_pcm24 = depth == WavPcmDepth::Pcm24;
    const std::size_t bytes_per_sample = is_pcm24 ? 3U : 2U;
    const std::int32_t scale = is_pcm24 ? 8'388'607 : 32'767;
    encoded.resize(samples.size() * bytes_per_sample);
    measured.resize(samples.size());
    for (std::size_t index = 0; index < samples.size(); ++index) {
        const std::int32_t value = quantize(samples[index], dither, scale);
        const std::uint32_t bits = static_cast<std::uint32_t>(value);
        const std::size_t offset = index * bytes_per_sample;
        encoded[offset] = std::byte{static_cast<unsigned char>(bits & 0xffU)};
        encoded[offset + 1] = std::byte{static_cast<unsigned char>((bits >> 8U) & 0xffU)};
        if (is_pcm24) {
            encoded[offset + 2] = std::byte{static_cast<unsigned char>((bits >> 16U) & 0xffU)};
        }
        measured[index] = static_cast<float>(value) / static_cast<float>(scale);
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

OfflineRenderResult OfflineWavRenderer::render(
    const std::string& sourcePath,
    const PlaybackAdjustment& adjustment,
    RenderByteSink& sink,
    const OfflineRenderCallbacks& callbacks,
    WavPcmDepth depth,
    std::string_view comment
) {
    const auto metadata = wav_comment_chunk(comment);
    // Zero is the shared playback contract for the complete remaining source.
    if (adjustment.trim_end_millis != 0
        && adjustment.trim_end_millis <= adjustment.trim_start_millis) {
        throw std::invalid_argument("offline render requires a non-empty selection");
    }
    PlaybackSession session(
        sourcePath,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    const std::uint64_t expected_frames = session.output_frame_count();
    const std::uint16_t bit_depth = static_cast<std::uint16_t>(depth);
    const std::uint16_t bytes_per_sample = static_cast<std::uint16_t>(bit_depth / 8U);
    const std::uint64_t bytes_per_frame = kChannels * bytes_per_sample;
    if (expected_frames > std::numeric_limits<std::uint64_t>::max() / bytes_per_frame) {
        throw std::invalid_argument("selection exceeds the WAV v1 size limit");
    }
    const std::uint64_t expected_data_bytes = expected_frames * bytes_per_frame;
    if (expected_data_bytes > std::numeric_limits<std::uint32_t>::max() - 36U - metadata.size()) {
        throw std::invalid_argument("selection exceeds the WAV v1 size limit");
    }
    const auto placeholder = wav_header(0, bit_depth);
    sink.write(placeholder);

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
        append_pcm(
            std::span<const float>(decoded.data(), sample_count),
            encoded,
            measured,
            dither,
            depth
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
    if (frame_count != expected_frames) {
        throw std::runtime_error(
            "offline render produced " + std::to_string(frame_count) + " frames; expected "
            + std::to_string(expected_frames)
        );
    }
    if (data_bytes > std::numeric_limits<std::uint32_t>::max() - 36U - metadata.size()) {
        throw std::runtime_error("rendered data exceeds the WAV v1 size limit");
    }
    if (!metadata.empty()) {
        sink.write(metadata);
    }
    sink.seek(0);
    const auto header = wav_header(
        static_cast<std::uint32_t>(data_bytes),
        bit_depth,
        static_cast<std::uint32_t>(metadata.size())
    );
    sink.write(header);
    report_progress(callbacks, 1.0);
    const OfflineLoudnessResult loudness = analyzer.result();
    return {
        .frame_count = frame_count,
        .size_bytes = kWavHeaderBytes + data_bytes + metadata.size(),
        .sample_rate = session.sample_rate(),
        .channel_count = session.channel_count(),
        .bit_depth = bit_depth,
        .integrated_lufs = loudness.integrated_lufs,
        .true_peak_dbtp = loudness.true_peak_dbtp,
    };
}

} // namespace echo::audio
