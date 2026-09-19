#include "echo/audio/offline_assembly_wav_renderer.hpp"
#include "echo/audio/offline_wav_renderer.hpp"
#include "echo/audio/playback.hpp"
#include "echo/audio/profiled_noise_reduction.hpp"
#include "echo/audio/spectrogram_detail.hpp"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <numbers>
#include <random>
#include <stdexcept>
#include <thread>

namespace {
using namespace echo::audio;
void require(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
class Sink final : public RenderByteSink {
  public:
    std::vector<std::byte> bytes;
    std::size_t cursor = 0;
    void write(std::span<const std::byte> data) override {
        bytes.resize(std::max(bytes.size(), cursor + data.size()));
        std::memcpy(bytes.data() + cursor, data.data(), data.size());
        cursor += data.size();
    }
    void seek(std::uint64_t offset) override {
        cursor = static_cast<std::size_t>(offset);
    }
};
float pcm(const Sink& sink, std::size_t sample) {
    const auto at = 44 + sample * 3;
    std::uint32_t bits = std::to_integer<unsigned>(sink.bytes[at])
                         | (std::to_integer<unsigned>(sink.bytes[at + 1]) << 8)
                         | (std::to_integer<unsigned>(sink.bytes[at + 2]) << 16);
    if (bits & 0x800000)
        bits |= 0xff000000;
    return static_cast<float>(static_cast<std::int32_t>(bits)) / 8388607.0F;
}
std::filesystem::path fixture() {
    const auto path =
        std::filesystem::temp_directory_path()
        / ("echo-workflow-contract-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".wav");
    std::ofstream file(path, std::ios::binary);
    const auto write = [&](auto value) {
        file.write(reinterpret_cast<const char*>(&value), sizeof(value));
    };
    constexpr std::uint32_t frames = 48'000 * 5, bytes = frames * 4;
    file.write("RIFF", 4);
    write(std::uint32_t(36 + bytes));
    file.write("WAVEfmt ", 8);
    write(std::uint32_t(16));
    write(std::uint16_t(1));
    write(std::uint16_t(2));
    write(std::uint32_t(48'000));
    write(std::uint32_t(192'000));
    write(std::uint16_t(4));
    write(std::uint16_t(16));
    file.write("data", 4);
    write(bytes);
    std::mt19937 random(971);
    std::uniform_real_distribution<double> noise(-0.03, 0.03);
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        const double time = static_cast<double>(frame) / 48'000;
        const double signal =
            noise(random) + 0.02 * std::sin(2 * std::numbers::pi * 180 * time)
            + (time >= 2 ? 0.25 * std::sin(2 * std::numbers::pi * 1000 * time) : 0);
        const auto sample = static_cast<std::int16_t>(signal * 32767);
        write(sample);
        write(static_cast<std::int16_t>(-sample));
    }
    return path;
}

std::vector<float> render(const std::string& path, echo::audio::PlaybackAdjustment adjustment) {
    echo::audio::PlaybackSession session(
        path,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    std::vector<float> result, buffer(4096 * session.channel_count());
    for (;;) {
        const auto frames = session.read(buffer.data(), 4096);
        result.insert(
            result.end(),
            buffer.begin(),
            buffer.begin() + static_cast<std::ptrdiff_t>(frames * session.channel_count())
        );
        if (!frames) {
            if (session.is_ended() || session.is_stopped())
                break;
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    }
    return result;
}

} // namespace
int main(int argc, char** argv) {
    using namespace echo::audio;
    const auto source = fixture();
    const auto prepared = source.string() + ".prepared.wav";
    const auto started = std::chrono::steady_clock::now();
    try {
        auto profile = learn_noise_profile(source.string(), 100, 900);
        profile.enabled = true;
        PlaybackAdjustment adjustment;
        adjustment.trim_start_millis = 200;
        adjustment.trim_end_millis = 4800;
        adjustment.fade_in_millis = 30;
        adjustment.fade_out_millis = 40;
        adjustment.profiled_noise_reduction = profile;
        adjustment.spectral_repair = {
            {.start_millis = 3000,
             .end_millis = 4000,
             .low_hertz = 800,
             .high_hertz = 1200,
             .attenuation_centibels = 1200,
             .time_feather_millis = 25,
             .frequency_feather_hertz = 50}
        };
        adjustment.gain_centibels = 250;
        adjustment.low_cut_hertz = 60;
        adjustment.de_click.enabled = true;
        adjustment.compressor.enabled = true;
        adjustment.limiter.enabled = true;
        adjustment.effect_chain = {
            EffectNodeKind::DeClick,
            EffectNodeKind::Equalizer,
            EffectNodeKind::Dynamics,
            EffectNodeKind::Master
        };
        adjustment.effect_chain_count = 4;
        adjustment.edit_segments = {
            {.source_start_millis = 200,
             .source_end_millis = 1500,
             .state = EditSegmentState::Audible},
            {.source_start_millis = 1500,
             .source_end_millis = 2000,
             .state = EditSegmentState::Hidden},
            {.source_start_millis = 2000,
             .source_end_millis = 2500,
             .state = EditSegmentState::Muted},
            {.source_start_millis = 2500,
             .source_end_millis = 4800,
             .state = EditSegmentState::Audible}
        };
        const auto preview = render(source.string(), adjustment);
        Sink output;
        double progress = 0;
        auto callbacks = OfflineRenderCallbacks{.progress = [&](double next) {
            require(next >= progress, "render progress regressed");
            progress = next;
        }};
        const auto result =
            OfflineWavRenderer::render(source.string(), adjustment, output, callbacks);
        require(
            result.frame_count == 4100 * 48 && preview.size() == result.frame_count * 2,
            "combined edits lost duration"
        );
        float maximum_error = 0;
        for (std::size_t sample = 0; sample < preview.size(); ++sample) {
            require(std::isfinite(preview[sample]), "non-finite combined output");
            maximum_error =
                std::max(maximum_error, std::abs(preview[sample] - pcm(output, sample)));
        }
        require(maximum_error < 0.000002F, "preview and delivered PCM disagree");
        require(progress == 1, "render never completed progress");
        {
            std::ofstream file(prepared, std::ios::binary);
            file.write(
                reinterpret_cast<const char*>(output.bytes.data()),
                static_cast<std::streamsize>(output.bytes.size())
            );
        }
        AssemblyMixPlan mix;
        mix.tracks.resize(8);
        for (std::size_t track = 0; track < mix.tracks.size(); ++track) {
            mix.tracks[track].gain_centibels = -600;
            mix.tracks[track].pan_percent =
                static_cast<std::int16_t>(static_cast<int>(track) * 20 - 70);
            for (std::uint64_t clip = 0; clip < 4; ++clip)
                mix.tracks[track].clips.push_back(
                    {.path = prepared,
                     .source_start_millis = 500,
                     .source_end_millis = 4000,
                     .timeline_start_millis = clip * 2500,
                     .fade_in_millis = 500,
                     .fade_out_millis = 500}
                );
        }
        Sink mixed;
        const auto mix_started = std::chrono::steady_clock::now();
        const auto mix_result = OfflineAssemblyWavRenderer::render(mix, mixed);
        const auto mix_ms = std::chrono::duration<double, std::milli>(
                                std::chrono::steady_clock::now() - mix_started
        )
                                .count();
        require(mix_result.frame_count == 11000 * 48, "overlapping 8-track mix duration changed");
        require(
            std::isfinite(mix_result.integrated_lufs) && mix_result.true_peak_dbtp <= 0.1F,
            "mix peak or loudness invalid"
        );
        mix.tracks[0].solo = true;
        for (std::size_t i = 1; i < mix.tracks.size(); ++i)
            mix.tracks[i].muted = true;
        Sink solo;
        const auto solo_result = OfflineAssemblyWavRenderer::render(mix, solo);
        require(
            solo_result.frame_count == mix_result.frame_count,
            "solo changed timeline duration"
        );
        require(
            solo_result.integrated_lufs < mix_result.integrated_lufs - 5,
            "solo/mute failed in overlapping mix"
        );
        bool cancelled = false;
        Sink partial;
        try {
            (void)OfflineAssemblyWavRenderer::render(mix, partial, {.cancelled = [&] {
                                                         return partial.bytes.size() > 50000;
                                                     }});
        } catch (const OfflineRenderCancelled&) {
            cancelled = true;
        }
        require(cancelled, "mix cancellation was ignored");
        const auto visual_source = argc > 1 ? std::string(argv[1]) : source.string();
        const auto visual_duration =
            argc > 2 ? static_cast<std::uint64_t>(std::stoull(argv[2])) : 5000;
        const auto spectral_start = std::chrono::steady_clock::now();
        const auto spectrum = build_spectrogram_detail(
            visual_source,
            {.end_millis = visual_duration, .columns = 1024, .rows = 384}
        );
        const auto spectral_ms = std::chrono::duration<double, std::milli>(
                                     std::chrono::steady_clock::now() - spectral_start
        )
                                     .count();
        require(spectrum.magnitudes.size() == 1024 * 384, "unbounded spectral image");
        std::cout << "{\"previewExportMaxError\":" << maximum_error
                  << ",\"tracks\":8,\"clips\":32,\"mixDurationMillis\":11000,\"mixRenderMillis\":"
                  << mix_ms << ",\"spectralSourceMillis\":" << visual_duration
                  << ",\"spectralMillis\":" << spectral_ms << ",\"totalMillis\":"
                  << std::chrono::duration<double, std::milli>(
                         std::chrono::steady_clock::now() - started
                     )
                         .count()
                  << "}\n";
        std::filesystem::remove(source);
        std::filesystem::remove(prepared);
        return 0;
    } catch (const std::exception& error) {
        std::filesystem::remove(source);
        std::filesystem::remove(prepared);
        std::cerr << error.what() << '\n';
        return 1;
    }
}
