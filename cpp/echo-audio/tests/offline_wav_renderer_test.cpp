#include "echo/audio/offline_flac_renderer.hpp"
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
    assert(result.frame_count == 24000);
    assert(result.size_bytes == sink.bytes().size());
    assert(progress == 1.0);
    assert(std::memcmp(sink.bytes().data(), "RIFF", 4) == 0);
    assert(std::memcmp(sink.bytes().data() + 8, "WAVE", 4) == 0);
    assert(std::memcmp(sink.bytes().data() + 36, "data", 4) == 0);
    assert(u32(sink.bytes(), 40) == sink.bytes().size() - 44);
    assert(u32(sink.bytes(), 24) == 48000);

    MemorySink pcm16_sink;
    const auto pcm16 = echo::audio::OfflineWavRenderer::render(
        source.string(),
        adjustment,
        pcm16_sink,
        {},
        echo::audio::WavPcmDepth::Pcm16
    );
    assert(pcm16.bit_depth == 16);
    assert(pcm16.size_bytes == pcm16_sink.bytes().size());
    assert(std::to_integer<std::uint8_t>(pcm16_sink.bytes()[34]) == 16);
    assert(pcm16_sink.bytes().size() < sink.bytes().size());

    MemorySink flac_sink;
    const auto flac =
        echo::audio::OfflineFlacRenderer::render(source.string(), adjustment, flac_sink);
    assert(flac.sample_rate == 48000);
    assert(flac.channel_count == 2);
    assert(flac.bit_depth == 24);
    assert(flac.frame_count == 24000);
    assert(flac.size_bytes == flac_sink.bytes().size());
    assert(std::memcmp(flac_sink.bytes().data(), "fLaC", 4) == 0);

    MemorySink latency_compensated_sink;
    auto latency_compensated = adjustment;
    latency_compensated.effect_chain = {
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::DeHum,
    };
    latency_compensated.effect_chain_count = 3;
    const auto compensated = echo::audio::OfflineWavRenderer::render(
        source.string(),
        latency_compensated,
        latency_compensated_sink
    );
    assert(compensated.frame_count == 24000);
    assert(compensated.size_bytes == latency_compensated_sink.bytes().size());

    MemorySink full_source_sink;
    auto full_source = latency_compensated;
    full_source.trim_start_millis = 0;
    full_source.trim_end_millis = 1000;
    const auto full_source_result =
        echo::audio::OfflineWavRenderer::render(source.string(), full_source, full_source_sink);
    assert(full_source_result.frame_count == 48'000);
    assert(full_source_result.size_bytes == full_source_sink.bytes().size());

    auto source_edited = full_source;
    source_edited.edit_segments = {
        {
            .source_start_millis = 0,
            .source_end_millis = 250,
            .state = echo::audio::EditSegmentState::Audible,
        },
        {
            .source_start_millis = 250,
            .source_end_millis = 500,
            .state = echo::audio::EditSegmentState::Hidden,
            .gap_after_millis = 100,
        },
        {
            .source_start_millis = 500,
            .source_end_millis = 750,
            .state = echo::audio::EditSegmentState::Muted,
        },
        {
            .source_start_millis = 750,
            .source_end_millis = 1000,
            .state = echo::audio::EditSegmentState::Audible,
        },
    };
    MemorySink edited_wav_sink;
    const auto edited_wav =
        echo::audio::OfflineWavRenderer::render(source.string(), source_edited, edited_wav_sink);
    assert(edited_wav.frame_count == 40'800);
    assert(edited_wav.size_bytes == edited_wav_sink.bytes().size());
    MemorySink edited_flac_sink;
    const auto edited_flac =
        echo::audio::OfflineFlacRenderer::render(source.string(), source_edited, edited_flac_sink);
    assert(edited_flac.frame_count == edited_wav.frame_count);
    assert(edited_flac.size_bytes == edited_flac_sink.bytes().size());

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
