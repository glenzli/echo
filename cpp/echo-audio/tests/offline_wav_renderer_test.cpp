#include "echo/audio/offline_wav_renderer.hpp"

#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <span>
#include <string>
#include <vector>

namespace {

class MemorySink final : public echo::audio::RenderByteSink {
  public:
    void write(std::span<const std::byte> bytes) override {
        if (cursor_ + bytes.size() > bytes_.size()) {
            bytes_.resize(cursor_ + bytes.size());
        }
        std::memcpy(bytes_.data() + cursor_, bytes.data(), bytes.size());
        cursor_ += bytes.size();
    }

    void seek(std::uint64_t offset) override {
        cursor_ = static_cast<std::size_t>(offset);
    }

    [[nodiscard]] const std::vector<std::byte>& bytes() const {
        return bytes_;
    }

  private:
    std::vector<std::byte> bytes_;
    std::size_t cursor_ = 0;
};

void append(std::string& target, const void* bytes, std::size_t size) {
    target.append(static_cast<const char*>(bytes), size);
}

std::string sine_wav() {
    constexpr std::uint32_t sample_rate = 24000;
    constexpr std::uint16_t channels = 1;
    constexpr std::uint16_t bits = 16;
    constexpr std::uint32_t frames = sample_rate;
    constexpr std::uint32_t data_bytes = frames * channels * bits / 8;
    std::string wav;
    append(wav, "RIFF", 4);
    const std::uint32_t riff_size = 36 + data_bytes;
    append(wav, &riff_size, 4);
    append(wav, "WAVEfmt ", 8);
    const std::uint32_t fmt_size = 16;
    const std::uint16_t format = 1;
    const std::uint32_t byte_rate = sample_rate * channels * bits / 8;
    const std::uint16_t block_align = channels * bits / 8;
    append(wav, &fmt_size, 4);
    append(wav, &format, 2);
    append(wav, &channels, 2);
    append(wav, &sample_rate, 4);
    append(wav, &byte_rate, 4);
    append(wav, &block_align, 2);
    append(wav, &bits, 2);
    append(wav, "data", 4);
    append(wav, &data_bytes, 4);
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        const double phase =
            2.0 * 3.14159265358979323846 * 440.0 * static_cast<double>(frame) / sample_rate;
        const auto sample = static_cast<std::int16_t>(std::sin(phase) * 8000.0);
        append(wav, &sample, 2);
    }
    return wav;
}

std::uint32_t u32(const std::vector<std::byte>& bytes, std::size_t offset) {
    return std::to_integer<std::uint32_t>(bytes[offset])
           | (std::to_integer<std::uint32_t>(bytes[offset + 1]) << 8U)
           | (std::to_integer<std::uint32_t>(bytes[offset + 2]) << 16U)
           | (std::to_integer<std::uint32_t>(bytes[offset + 3]) << 24U);
}

} // namespace

int main() {
    const auto source =
        std::filesystem::temp_directory_path()
        / ("echo-offline-render-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".wav");
    {
        std::ofstream output(source, std::ios::binary);
        const std::string wav = sine_wav();
        output.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }

    MemorySink sink;
    double progress = 0.0;
    const echo::audio::PlaybackAdjustment adjustment{
        .trim_start_millis = 250,
        .trim_end_millis = 750,
        .fade_in_millis = 50,
        .fade_out_millis = 50,
        .gain_centibels = -300,
    };
    const auto result = echo::audio::OfflineWavRenderer::render(
        source.string(),
        adjustment,
        sink,
        {.progress = [&progress](double value) { progress = value; }}
    );
    assert(result.sample_rate == 48000);
    assert(result.channel_count == 2);
    assert(result.bit_depth == 24);
    assert(result.frame_count >= 23900 && result.frame_count <= 24100);
    assert(result.size_bytes == sink.bytes().size());
    assert(progress == 1.0);
    assert(std::memcmp(sink.bytes().data(), "RIFF", 4) == 0);
    assert(std::memcmp(sink.bytes().data() + 8, "WAVE", 4) == 0);
    assert(std::memcmp(sink.bytes().data() + 36, "data", 4) == 0);
    assert(u32(sink.bytes(), 40) == sink.bytes().size() - 44);
    assert(u32(sink.bytes(), 24) == 48000);

    MemorySink cancelled_sink;
    bool did_cancel = false;
    try {
        [[maybe_unused]] const auto cancelled_result = echo::audio::OfflineWavRenderer::render(
            source.string(),
            adjustment,
            cancelled_sink,
            {.cancelled = [] { return true; }}
        );
    } catch (const echo::audio::OfflineRenderCancelled&) {
        did_cancel = true;
    }
    assert(did_cancel);
    std::filesystem::remove(source);
}
