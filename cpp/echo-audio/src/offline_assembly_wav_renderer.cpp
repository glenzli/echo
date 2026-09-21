#include "echo/audio/offline_assembly_wav_renderer.hpp"
#include "echo/audio/export_metadata.hpp"

#include "echo/audio/adjustment.hpp"
#include "echo/audio/assembly_mixer.hpp"
#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <chrono>
#include <cmath>
#include <cstring>
#include <limits>
#include <memory>
#include <numbers>
#include <ranges>
#include <span>
#include <stdexcept>
#include <thread>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kSampleRate = 48'000;
constexpr std::uint16_t kChannels = 2;
constexpr std::uint16_t kBitDepth = 24;
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
wav_header(std::uint32_t data_size, std::uint32_t metadata_size = 0) {
    constexpr std::uint16_t bytes_per_sample = kBitDepth / 8U;
    std::array<std::byte, kWavHeaderBytes> header{};
    put_fourcc(header, 0, "RIFF");
    put_little_endian<std::uint32_t>(header, 4, 36U + data_size + metadata_size);
    put_fourcc(header, 8, "WAVE");
    put_fourcc(header, 12, "fmt ");
    put_little_endian<std::uint32_t>(header, 16, 16U);
    put_little_endian<std::uint16_t>(header, 20, 1U);
    put_little_endian<std::uint16_t>(header, 22, kChannels);
    put_little_endian<std::uint32_t>(header, 24, kSampleRate);
    put_little_endian<std::uint32_t>(header, 28, kSampleRate * kChannels * bytes_per_sample);
    put_little_endian<std::uint16_t>(header, 32, kChannels * bytes_per_sample);
    put_little_endian<std::uint16_t>(header, 34, kBitDepth);
    put_fourcc(header, 36, "data");
    put_little_endian<std::uint32_t>(header, 40, data_size);
    return header;
}

void report_progress(const OfflineRenderCallbacks& callbacks, double value) {
    if (callbacks.progress) {
        callbacks.progress(std::clamp(value, 0.0, 1.0));
    }
}

class Dither {
  public:
    float tpdf() {
        return (uniform() - uniform()) / 8'388'608.0F;
    }

  private:
    float uniform() {
        state_ ^= state_ << 13U;
        state_ ^= state_ >> 17U;
        state_ ^= state_ << 5U;
        return static_cast<float>(state_ & 0x00ff'ffffU) / 16'777'216.0F;
    }

    std::uint32_t state_ = 0x4153'4d42U;
};

void encode_pcm24(
    std::span<const float> samples,
    std::vector<std::byte>& encoded,
    std::vector<float>& measured,
    Dither& dither
) {
    constexpr std::int32_t scale = 8'388'607;
    encoded.resize(samples.size() * 3U);
    measured.resize(samples.size());
    for (std::size_t index = 0; index < samples.size(); ++index) {
        const float prepared = std::clamp(
            samples[index] + dither.tpdf(),
            -1.0F,
            static_cast<float>(scale) / 8'388'608.0F
        );
        const auto value = std::clamp(
            static_cast<std::int32_t>(std::lrint(prepared * static_cast<float>(scale))),
            -8'388'608,
            scale
        );
        const auto bits = static_cast<std::uint32_t>(value);
        const std::size_t offset = index * 3U;
        encoded[offset] = std::byte{static_cast<unsigned char>(bits & 0xffU)};
        encoded[offset + 1] = std::byte{static_cast<unsigned char>((bits >> 8U) & 0xffU)};
        encoded[offset + 2] = std::byte{static_cast<unsigned char>((bits >> 16U) & 0xffU)};
        measured[index] = static_cast<float>(value) / static_cast<float>(scale);
    }
}

} // namespace

OfflineRenderResult OfflineAssemblyWavRenderer::render(
    const AssemblyMixPlan& plan,
    RenderByteSink& sink,
    const OfflineRenderCallbacks& callbacks,
    std::string_view comment
) {
    const auto metadata = wav_comment_chunk(comment);
    AssemblyMixer mixer(plan);
    const auto expected_frames = mixer.frame_count();
    constexpr std::uint64_t bytes_per_frame = kChannels * (kBitDepth / 8U);
    if (expected_frames
        > (std::numeric_limits<std::uint32_t>::max() - 36U - metadata.size()) / bytes_per_frame) {
        throw std::invalid_argument("assembly exceeds the WAV v1 size limit");
    }

    sink.write(wav_header(0));
    OfflineLoudnessAnalyzer analyzer(kSampleRate, kChannels);
    std::vector<float> measured;
    std::vector<std::byte> encoded;
    Dither dither;
    std::uint64_t cursor = 0;
    std::uint64_t data_bytes = 0;
    while (cursor < expected_frames) {
        const auto mix = mixer.next(callbacks);
        const auto frames = mix.size() / kChannels;
        encode_pcm24(mix, encoded, measured, dither);
        sink.write(encoded);
        analyzer.process_interleaved(measured.data(), frames, kChannels);
        cursor += frames;
        data_bytes += encoded.size();
        report_progress(
            callbacks,
            static_cast<double>(cursor) / static_cast<double>(expected_frames)
        );
    }
    if (!metadata.empty()) {
        sink.write(metadata);
    }
    sink.seek(0);
    sink.write(wav_header(
        static_cast<std::uint32_t>(data_bytes),
        static_cast<std::uint32_t>(metadata.size())
    ));
    report_progress(callbacks, 1.0);
    const auto loudness = analyzer.result();
    return {
        .frame_count = expected_frames,
        .size_bytes = kWavHeaderBytes + data_bytes + metadata.size(),
        .sample_rate = kSampleRate,
        .channel_count = kChannels,
        .bit_depth = kBitDepth,
        .integrated_lufs = loudness.integrated_lufs,
        .true_peak_dbtp = loudness.true_peak_dbtp,
    };
}

} // namespace echo::audio
