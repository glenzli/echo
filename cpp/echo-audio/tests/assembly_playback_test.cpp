#include "echo/audio/assembly_mixer.hpp"
#include "echo/audio/assembly_playback.hpp"
#include "echo/audio/offline_assembly_wav_renderer.hpp"
#include <array>
#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <limits>
#include <span>
#include <string>
#include <thread>
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

} // namespace

using namespace echo::audio;
using Clock = std::chrono::steady_clock;
using namespace std::chrono_literals;
template <typename Predicate> void until(Predicate predicate) {
    const auto deadline = Clock::now() + 4s;
    while (!predicate()) {
        assert(Clock::now() < deadline);
        std::this_thread::sleep_for(1ms);
    }
}
void parity(const AssemblyMixPlan& plan) {
    MemorySink sink;
    const auto rendered = OfflineAssemblyWavRenderer::render(plan, sink);
    AssemblyPlaybackSession stream(plan, false);
    std::vector<float> output;
    std::array<float, 2048> buffer{};
    std::size_t iteration = 0;
    const auto deadline = Clock::now() + 4s;
    while (!stream.is_ended() || stream.buffered_frames()) {
        assert(Clock::now() < deadline && stream.error().empty());
        const auto count = stream.read(buffer.data(), iteration++ % 2 ? 137 : 1024);
        output.insert(
            output.end(),
            buffer.begin(),
            buffer.begin() + static_cast<std::ptrdiff_t>(count * 2)
        );
        if (!count)
            std::this_thread::sleep_for(1ms);
    }
    assert(output.size() == rendered.frame_count * 2);
    for (std::size_t frame = 0; frame < rendered.frame_count; ++frame)
        for (std::size_t channel = 0; channel < 2; ++channel)
            assert(
                std::abs(output[frame * 2 + channel] - pcm24(sink.bytes_, frame, channel))
                < 0.000001F
            );
    assert(stream.position_millis() == stream.duration_millis());
}
int main() {
    const auto root =
        std::filesystem::temp_directory_path()
        / ("echo-stream-contract-" + std::to_string(Clock::now().time_since_epoch().count()));
    std::filesystem::create_directories(root);
    const auto source = root / "source.wav";
    {
        std::ofstream file(source, std::ios::binary);
        file << constant_wav(0.3F);
    }
    AssemblyClipSource clip{.path = source.string(), .source_end_millis = 1000};
    AssemblyMixPlan plan;
    plan.tracks = {
        AssemblyTrackMix{.clips = {clip}},
        AssemblyTrackMix{.pan_percent = -50, .clips = {clip}}
    };
    plan.tracks[1].clips[0].timeline_start_millis = 250;
    plan.tracks[0].clips[0].fade_in_millis = 120;
    plan.tracks[0].clips[0].fade_out_millis = 90;
    plan.tracks[0].clips[0].gain_envelope_enabled = true;
    plan.tracks[0].clips[0].gain_envelope = {{0, 0}, {500, -600}, {1000, 0}};
    parity(plan);
    plan.master_gain_centibels = 1200;
    parity(plan); // Actively limiting; same partition/state as export.
    plan.render_start_millis = 125;
    plan.render_end_millis = 999;
    parity(plan);
    plan.tracks[1].solo = true;
    parity(plan);

    // A non-canonical WAV layout still takes the general decoder path.
    auto extra = constant_wav(-0.3F);
    const std::uint32_t extraSize = static_cast<std::uint32_t>(extra.size());
    extra.insert(12, std::string("JUNK\0\0\0\0", 8));
    std::memcpy(extra.data() + 4, &extraSize, sizeof(extraSize));
    const auto extended = root / "extended.wav";
    {
        std::ofstream file(extended, std::ios::binary);
        file << extra;
    }
    AssemblyMixPlan fallback;
    fallback.limiter_enabled = false;
    fallback.tracks = {AssemblyTrackMix{.clips = {clip}}};
    fallback.tracks[0].clips[0].path = extended.string();
    MemorySink negative;
    (void)OfflineAssemblyWavRenderer::render(fallback, negative);
    assert(std::abs(pcm24(negative.bytes_, 123, 0) + 0.3F) < 0.001F);
    // Re-read the actual PCM24 output, including signed samples and exact seek.
    const auto pcm = root / "prepared.wav";
    {
        std::ofstream file(pcm, std::ios::binary);
        file.write(
            reinterpret_cast<const char*>(negative.bytes_.data()),
            static_cast<std::streamsize>(negative.bytes_.size())
        );
    }
    fallback.tracks[0].clips[0].path = pcm.string();
    fallback.render_start_millis = 137;
    parity(fallback);
    AssemblyMixer direct(fallback);
    direct.seek(11);
    assert(std::abs(direct.next()[0] + 0.3F) < 0.001F);
    // An otherwise canonical header must never hide truncated source data.
    std::filesystem::resize_file(pcm, 100);
    bool truncated = false;
    try {
        AssemblyMixer broken(fallback);
        (void)broken.next();
    } catch (const std::runtime_error&) {
        truncated = true;
    }
    assert(truncated);

    AssemblyMixPlan longPlan;
    longPlan.limiter_enabled = false;
    longPlan.tracks = {AssemblyTrackMix{.clips = {clip, clip}}};
    longPlan.tracks[0].clips[1].timeline_start_millis = 14399000;
    auto started = Clock::now();
    AssemblyPlaybackSession stream(longPlan, false);
    until([&] { return stream.buffered_frames() >= 4096; });
    const auto firstReady =
        std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now() - started).count();
    assert(firstReady < 2000 && stream.duration_millis() == 14400000);
    until([&] { return stream.buffered_frames() == AssemblyPlaybackSession::capacity_frames; });
    std::array<float, 2048> buffer{};
    stream.pause();
    assert(stream.read(buffer.data(), 1024) == 0);
    stream.seek(2000);
    until([&] { return stream.position_millis() == 2000; });
    stream.resume();
    until([&] { return stream.buffered_frames() >= 4096; });
    assert(stream.read(buffer.data(), 1024) == 1024);
    for (float sample : buffer)
        assert(sample == 0); // Buffered audio before seek must not leak into this gap.
    for (int i = 0; i < 40; ++i) {
        stream.seek(0);
        stream.seek(700);
        stream.seek(500);
        until([&] { return stream.position_millis() == 500; });
        until([&] { return stream.buffered_frames() >= 4096; });
        assert(stream.read(buffer.data(), 1024) == 1024);
        assert(std::abs(buffer[0] - 0.3F) < 0.001F);
    }
    stream.seek(std::numeric_limits<std::uint64_t>::max());
    until([&] { return stream.is_ended() && stream.buffered_frames() == 0; });
    assert(stream.position_millis() == stream.duration_millis());
    stream.seek(0);
    until([&] { return stream.position_millis() == 0 && stream.buffered_frames() > 0; });
    started = Clock::now();
    stream.stop();
    assert(Clock::now() - started < 500ms && stream.read(buffer.data(), 1024) == 0);
    auto invalid = longPlan;
    invalid.tracks[0].clips[0].path = (root / "missing.wav").string();
    AssemblyPlaybackSession failed(invalid);
    until([&] { return !failed.error().empty(); });
    failed.stop();
    invalid = longPlan;
    invalid.tracks[0].clips[0].timeline_start_millis = std::numeric_limits<std::uint64_t>::max();
    bool rejected = false;
    try {
        AssemblyMixer mixer(invalid);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);

    AssemblyMixPlan dense;
    dense.limiter_enabled = false;
    clip.gain_centibels = -2400;
    dense.tracks.assign(
        8,
        AssemblyTrackMix{
            .gain_centibels = -2400,
            .clips = std::vector<AssemblyClipSource>(32, clip)
        }
    );
    started = Clock::now();
    AssemblyPlaybackSession crowded(dense, false);
    until([&] { return crowded.buffered_frames() >= 4096 || !crowded.error().empty(); });
    assert(crowded.error().empty());
    const auto denseReady =
        std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now() - started).count();
    assert(denseReady < 1000);
    assert(crowded.read(buffer.data(), 1024) == 1024);
    const float denseSample = 256 * 0.3F * std::pow(10.0F, -48.0F / 20.0F);
    assert(std::abs(buffer[0] - denseSample) < 0.001F);
    crowded.seek(500);
    until([&] { return crowded.position_millis() == 500 && crowded.buffered_frames() >= 4096; });
    assert(crowded.read(buffer.data(), 1024) == 1024);
    assert(std::abs(buffer[0] - denseSample) < 0.001F);
    started = Clock::now();
    crowded.stop();
    assert(Clock::now() - started < 500ms);
    std::filesystem::remove_all(root);
    std::cout << "Offline/stream parity, 4-hour bounded playback (first buffer " << firstReady
              << " ms), 256 overlapping clips (first buffer " << denseReady
              << " ms), seek, pause, stop and decode failure passed\n";
}
