#include "echo/audio/audio_export.hpp"
#include "echo/audio/decode.hpp"
#include "echo/audio/playback.hpp"
#include <algorithm>
#include <cassert>
#include <chrono>
#include <cmath>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <span>
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

std::string sine_wav(std::uint32_t frames = 24000) {
    constexpr std::uint32_t sample_rate = 24000;
    constexpr std::uint16_t channels = 1;
    constexpr std::uint16_t bits = 16;
    const std::uint32_t data_bytes = frames * channels * bits / 8;
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
        double plosive = 0.0;
        if (frame >= 8'000 && frame < 10'400) {
            const double progress = static_cast<double>(frame - 8'000) / 2'400.0;
            const double burst_phase =
                2.0 * 3.14159265358979323846 * 85.0 * static_cast<double>(frame) / sample_rate;
            plosive = std::exp(-3.2 * progress) * std::sin(burst_phase) * 20'000.0;
        }
        const auto sample = static_cast<std::int16_t>(
            std::clamp(std::sin(phase) * 3'000.0 + plosive, -32'767.0, 32'767.0)
        );
        append(wav, &sample, 2);
    }
    return wav;
}
std::uint64_t decode_frames(const std::string& path, std::uint64_t seek_millis = 0) {
    echo::audio::PlaybackSession playback(
        path,
        {},
        {.apply_output_guard = false, .collect_metering = false}
    );
    if (seek_millis)
        playback.seek(seek_millis);
    std::vector<float> samples(8192);
    std::uint64_t frames = 0;
    const auto started = std::chrono::steady_clock::now();
    for (;;) {
        const auto count = playback.read(samples.data(), 4096);
        for (std::size_t i = 0; i < count * 2; ++i)
            assert(std::isfinite(samples[i]));
        frames += count;
        if (playback.is_ended() && !playback.buffered_frames())
            break;
        assert(std::chrono::steady_clock::now() - started < std::chrono::seconds(10));
        if (!count)
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    assert(frames > 0);
    return frames;
}

} // namespace
int main(int argc, char** argv) {
    if (argc > 1) {
        for (int i = 1; i < argc; ++i) {
            const auto metadata = echo::audio::probe(argv[i]);
            assert(metadata.has_audio);
            assert(metadata.duration_millis > 0);
            const auto frames = decode_frames(argv[i]);
            const auto sought = decode_frames(argv[i], metadata.duration_millis / 2);
            assert(std::abs(static_cast<double>(sought) - static_cast<double>(frames) / 2) < 6000);
            std::cerr << std::filesystem::path(argv[i]).filename().string() << " full=" << frames
                      << " seek=" << sought << " duration=" << metadata.duration_millis << "\n";
            MemorySink delivery;
            const auto exported = echo::audio::AudioExporter::render(argv[i], {}, delivery, {});
            assert(exported.frame_count == frames);
            MemorySink trimmed;
            const auto selection = echo::audio::AudioExporter::render(
                argv[i],
                {.trim_start_millis = metadata.duration_millis / 2},
                trimmed,
                {}
            );
            assert(selection.frame_count == frames - (metadata.duration_millis / 2) * 48);
            std::cout << std::filesystem::path(argv[i]).filename().string() << " "
                      << metadata.codec_name << " " << frames << " frames\n";
        }
        return 0;
    }
    const auto root =
        std::filesystem::temp_directory_path()
        / ("echo-export-profiles-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    std::filesystem::create_directories(root);
    const auto source = root / "source.wav";
    {
        std::ofstream file(source, std::ios::binary);
        const auto bytes = sine_wav();
        file.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
    }
    const std::string comment = "Echo source disclosure: "
                                "{\"schema\":\"echo.source-disclosure.v1\",\"scope\":\"referenced_"
                                "sources\",\"kinds\":[\"ai_generated\"]}";
    // Use a valid bounded comment through the production validator.
    for (const auto* format :
         {"wav_pcm16", "wav_pcm24", "wav_float32", "flac24", "mp3", "aac_m4a"}) {
        for (const auto rate : {44100U, 48000U, 96000U}) {
            if (rate == 96000 && (std::string(format) == "mp3" || std::string(format) == "aac_m4a"))
                continue;
            echo::audio::AudioExportProfile profile{
                .format = format,
                .sample_rate = rate,
                .channels = rate == 48000 ? 2U : 1U
            };
            if (rate == 48000) {
                profile.memory_notes = "第一次叫爸爸\nA quiet memory";
                profile.memory_place = "外婆家阳台";
                profile.memory_time = "大约 2020 年夏天";
            }
            MemorySink sink;
            const auto result =
                echo::audio::AudioExporter::render(source.string(), {}, sink, profile, {}, comment);
            assert(result.frame_count == rate);
            assert(result.size_bytes == sink.bytes().size());
            const auto destination =
                root
                / (std::string(format) + "-" + std::to_string(rate) + "." + profile.extension());
            {
                std::ofstream file(destination, std::ios::binary);
                file.write(
                    reinterpret_cast<const char*>(sink.bytes().data()),
                    static_cast<std::streamsize>(sink.bytes().size())
                );
            }
            const auto probe = echo::audio::probe(destination.string());
            assert(probe.sample_rate == rate);
            assert(probe.channel_count == profile.channels);
            assert(probe.has_audio);
            bool metadata = false;
            for (const auto& entry : probe.metadata)
                if (entry.key == "comment") {
                    assert(entry.value.starts_with(comment));
                    if (rate == 48000) {
                        assert(entry.value.find(profile.memory_notes) != std::string::npos);
                        assert(entry.value.find(profile.memory_place) != std::string::npos);
                        assert(entry.value.find(profile.memory_time) != std::string::npos);
                    } else
                        assert(entry.value == comment);
                    metadata = true;
                }
            assert(metadata);
            const auto decoded = decode_frames(destination.string());
            assert(std::abs(static_cast<double>(decoded) - 48000.0) < 2400);
            std::cout << format << " " << rate << " " << profile.channels << " metadata verified\n";
        }
    }
    MemorySink cancelled;
    bool interrupted = false;
    try {
        echo::audio::AudioExporter::render(source.string(), {}, cancelled, {}, {.cancelled = [] {
                                               return true;
                                           }});
    } catch (const echo::audio::OfflineRenderCancelled&) {
        interrupted = true;
    }
    assert(interrupted);
    std::filesystem::remove_all(root);
}
