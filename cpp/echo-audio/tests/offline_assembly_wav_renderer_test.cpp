#include "echo/audio/decode.hpp"
#include "echo/audio/offline_assembly_wav_renderer.hpp"

#include <algorithm>
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

    std::vector<std::byte> bytes_;
    std::size_t cursor_ = 0;
};

void append(std::string& target, const void* bytes, std::size_t size) {
    target.append(static_cast<const char*>(bytes), size);
}

std::string constant_wav(float amplitude) {
    constexpr std::uint32_t sample_rate = 48'000;
    constexpr std::uint16_t channels = 2;
    constexpr std::uint16_t bits = 16;
    constexpr std::uint32_t frames = sample_rate;
    constexpr std::uint32_t data_bytes = frames * channels * bits / 8U;
    std::string wav;
    append(wav, "RIFF", 4);
    const std::uint32_t riff_size = 36U + data_bytes;
    append(wav, &riff_size, 4);
    append(wav, "WAVEfmt ", 8);
    const std::uint32_t fmt_size = 16;
    const std::uint16_t format = 1;
    const std::uint32_t byte_rate = sample_rate * channels * bits / 8U;
    const std::uint16_t block_align = channels * bits / 8U;
    append(wav, &fmt_size, 4);
    append(wav, &format, 2);
    append(wav, &channels, 2);
    append(wav, &sample_rate, 4);
    append(wav, &byte_rate, 4);
    append(wav, &block_align, 2);
    append(wav, &bits, 2);
    append(wav, "data", 4);
    append(wav, &data_bytes, 4);
    const auto sample = static_cast<std::int16_t>(std::lrint(amplitude * 32'767.0F));
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        append(wav, &sample, 2);
        append(wav, &sample, 2);
    }
    return wav;
}

float pcm24(const std::vector<std::byte>& bytes, std::size_t frame, std::size_t channel) {
    const std::size_t offset = 44U + (frame * 2U + channel) * 3U;
    std::uint32_t raw = std::to_integer<std::uint32_t>(bytes[offset])
                        | (std::to_integer<std::uint32_t>(bytes[offset + 1]) << 8U)
                        | (std::to_integer<std::uint32_t>(bytes[offset + 2]) << 16U);
    if ((raw & 0x0080'0000U) != 0U) {
        raw |= 0xff00'0000U;
    }
    return static_cast<float>(static_cast<std::int32_t>(raw)) / 8'388'607.0F;
}

void expect_close(float actual, float expected) {
    assert(std::abs(actual - expected) < 0.004F);
}

} // namespace

int main() {
    const auto root =
        std::filesystem::temp_directory_path()
        / ("echo-assembly-render-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    std::filesystem::create_directories(root);
    const auto first = root / "first.wav";
    const auto second = root / "second.wav";
    {
        std::ofstream output(first, std::ios::binary);
        const auto wav = constant_wav(0.2F);
        output.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }
    {
        std::ofstream output(second, std::ios::binary);
        const auto wav = constant_wav(0.1F);
        output.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }

    echo::audio::AssemblyMixPlan plan;
    plan.limiter_enabled = false;
    echo::audio::AssemblyTrackMix first_track;
    first_track.clips.push_back({
        .path = first.string(),
        .source_start_millis = 0,
        .source_end_millis = 1'000,
        .timeline_start_millis = 0,
    });
    echo::audio::AssemblyTrackMix second_track;
    second_track.clips.push_back({
        .path = second.string(),
        .source_start_millis = 0,
        .source_end_millis = 1'000,
        .timeline_start_millis = 500,
    });
    plan.tracks = {first_track, second_track};

    MemorySink sink;
    const auto result = echo::audio::OfflineAssemblyWavRenderer::render(plan, sink);
    assert(result.frame_count == 72'000);
    assert(result.sample_rate == 48'000);
    assert(result.channel_count == 2);
    assert(result.bit_depth == 24);
    assert(result.size_bytes == sink.bytes_.size());
    expect_close(pcm24(sink.bytes_, 12'000, 0), 0.2F);
    expect_close(pcm24(sink.bytes_, 36'000, 0), 0.3F);
    expect_close(pcm24(sink.bytes_, 60'000, 0), 0.1F);

    MemorySink marked;
    const std::string comment =
        R"(Echo source disclosure: {"schema":"echo.source-disclosure.v1","scope":"referenced_sources","kinds":["ai_generated"]})";
    const auto marked_result =
        echo::audio::OfflineAssemblyWavRenderer::render(plan, marked, {}, comment);
    assert(marked_result.size_bytes == marked.bytes_.size());
    assert(std::equal(sink.bytes_.begin() + 44, sink.bytes_.end(), marked.bytes_.begin() + 44));
    const auto marked_path = root / "marked.wav";
    {
        std::ofstream file(marked_path, std::ios::binary);
        file.write(
            reinterpret_cast<const char*>(marked.bytes_.data()),
            static_cast<std::streamsize>(marked.bytes_.size())
        );
    }
    bool found = false;
    for (const auto& entry : echo::audio::probe(marked_path.string()).metadata) {
        if (entry.key == "comment") {
            assert(entry.value == comment);
            found = true;
        }
    }
    assert(found);

    auto faded = plan;
    faded.tracks[0].clips[0].fade_in_millis = 500;
    faded.tracks[0].clips[0].fade_in_curve = echo::audio::AssemblyFadeCurve::Linear;
    faded.tracks[1].muted = true;
    MemorySink faded_sink;
    const auto faded_result = echo::audio::OfflineAssemblyWavRenderer::render(faded, faded_sink);
    assert(faded_result.frame_count == 72'000);
    expect_close(pcm24(faded_sink.bytes_, 6'000, 0), 0.05F);
    expect_close(pcm24(faded_sink.bytes_, 36'000, 0), 0.2F);
    expect_close(pcm24(faded_sink.bytes_, 60'000, 0), 0.0F);

    auto automated = plan;
    automated.tracks.resize(1);
    auto& clip = automated.tracks[0].clips[0];
    clip.gain_envelope_enabled = true;
    clip.gain_envelope = {{0, 0}, {500, -1200}, {1000, 0}};
    MemorySink envelope_sink;
    [[maybe_unused]] const auto envelope_result =
        echo::audio::OfflineAssemblyWavRenderer::render(automated, envelope_sink);
    expect_close(pcm24(envelope_sink.bytes_, 12000, 0), 0.2F * std::pow(10.0F, -6.0F / 20.0F));
    expect_close(pcm24(envelope_sink.bytes_, 24000, 0), 0.2F * std::pow(10.0F, -12.0F / 20.0F));
    // A bounded preview must retain the full clip's source coordinate and fades.
    automated.render_start_millis = 250;
    automated.render_end_millis = 750;
    MemorySink range_sink;
    const auto range_result =
        echo::audio::OfflineAssemblyWavRenderer::render(automated, range_sink);
    assert(range_result.frame_count == 24000);
    for (std::size_t frame = 0; frame < 24000; frame += 100)
        expect_close(
            pcm24(range_sink.bytes_, frame, 0),
            pcm24(envelope_sink.bytes_, frame + 12000, 0)
        );
    automated.render_start_millis = 0;
    automated.render_end_millis = 0;
    auto right = clip;
    clip.source_end_millis = 500;
    right.source_start_millis = 500;
    right.timeline_start_millis = 500;
    automated.tracks[0].clips.push_back(right);
    MemorySink split_sink;
    [[maybe_unused]] const auto split_result =
        echo::audio::OfflineAssemblyWavRenderer::render(automated, split_sink);
    for (std::size_t frame = 0; frame < 48000; frame += 100)
        expect_close(pcm24(split_sink.bytes_, frame, 0), pcm24(envelope_sink.bytes_, frame, 0));
    for (auto& part : automated.tracks[0].clips)
        part.gain_envelope_enabled = false;
    MemorySink bypass_sink;
    [[maybe_unused]] const auto bypass_result =
        echo::audio::OfflineAssemblyWavRenderer::render(automated, bypass_sink);
    expect_close(pcm24(bypass_sink.bytes_, 24000, 0), 0.2F);
    automated.tracks[0].clips[0].gain_envelope[1].source_millis = 0;
    bool rejected = false;
    try {
        MemorySink invalid;
        [[maybe_unused]] const auto invalid_result =
            echo::audio::OfflineAssemblyWavRenderer::render(automated, invalid);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);

    // A near-four-hour, 256-clip session prepares only the selected output window.
    echo::audio::AssemblyMixPlan large;
    large.limiter_enabled = false;
    for (std::size_t track_index = 0; track_index < 8; ++track_index) {
        echo::audio::AssemblyTrackMix track;
        for (std::size_t clip_index = 0; clip_index < 32; ++clip_index) {
            echo::audio::AssemblyClipSource piece;
            piece.path = first.string();
            piece.source_end_millis = 1000;
            piece.timeline_start_millis = (track_index * 32 + clip_index) * 55000;
            piece.gain_envelope_enabled = true;
            piece.gain_envelope = {{0, 0}, {1000, -1200}};
            track.clips.push_back(piece);
        }
        large.tracks.push_back(track);
    }
    large.render_start_millis = 255 * 55000 + 250;
    large.render_end_millis = large.render_start_millis + 50;
    MemorySink large_sink;
    const auto large_result = echo::audio::OfflineAssemblyWavRenderer::render(large, large_sink);
    assert(large_result.frame_count == 2400);
    expect_close(pcm24(large_sink.bytes_, 0, 0), 0.2F * std::pow(10.0F, -3.0F / 20.0F));

    std::filesystem::remove_all(root);
    return 0;
}
