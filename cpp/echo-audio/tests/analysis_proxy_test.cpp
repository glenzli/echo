#include "echo/audio/analysis_proxy.hpp"

#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <string>

namespace {

void append(std::string& target, const void* bytes, std::size_t size) {
    target.append(static_cast<const char*>(bytes), size);
}

std::string sine_wav() {
    constexpr std::uint32_t sample_rate = 24000;
    constexpr std::uint16_t channels = 2;
    constexpr std::uint16_t bits = 16;
    constexpr std::uint32_t frames = sample_rate * 2;
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
        append(wav, &sample, 2);
    }
    return wav;
}

} // namespace

int main() {
    const auto suffix = std::to_string(std::chrono::steady_clock::now().time_since_epoch().count());
    const auto source =
        std::filesystem::temp_directory_path() / ("echo-analysis-proxy-source-" + suffix + ".wav");
    const auto output =
        std::filesystem::temp_directory_path() / ("echo-analysis-proxy-output-" + suffix + ".wav");
    {
        std::ofstream file(source, std::ios::binary);
        const std::string wav = sine_wav();
        file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }
    const auto result =
        echo::audio::build_analysis_proxy(source.string(), output.string(), 500, 1500);
    assert(result.sample_rate == 16000);
    assert(result.channel_count == 1);
    assert(result.frame_count >= 15900 && result.frame_count <= 16100);
    assert(result.size_bytes == std::filesystem::file_size(output));
    assert(result.size_bytes < 33000);
    const auto full = echo::audio::build_analysis_proxy(source.string(), output.string(), 0, 2000);
    assert(full.frame_count == 32000);
    const auto tail =
        echo::audio::build_analysis_proxy(source.string(), output.string(), 1500, 2000);
    assert(tail.frame_count == 8000);
    std::filesystem::remove(source);
    std::filesystem::remove(output);
}
