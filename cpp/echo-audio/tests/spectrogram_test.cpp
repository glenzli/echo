#include <echo/audio/spectrogram.hpp>

#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <string>

namespace {

std::string sine_wav() {
    constexpr std::uint32_t sample_rate = 48'000;
    constexpr std::uint32_t frames = sample_rate * 2;
    constexpr std::uint16_t channels = 1;
    constexpr std::uint16_t bits = 16;
    const std::uint32_t data_bytes = frames * channels * bits / 8;
    std::string wav;
    const auto append = [&wav](const void* bytes, std::size_t size) {
        wav.append(static_cast<const char*>(bytes), size);
    };
    append("RIFF", 4);
    const std::uint32_t riff_size = 36 + data_bytes;
    append(&riff_size, 4);
    append("WAVEfmt ", 8);
    const std::uint32_t fmt_size = 16;
    const std::uint16_t format = 1;
    append(&fmt_size, 4);
    append(&format, 2);
    append(&channels, 2);
    append(&sample_rate, 4);
    const std::uint32_t byte_rate = sample_rate * channels * bits / 8;
    const std::uint16_t block_align = channels * bits / 8;
    append(&byte_rate, 4);
    append(&block_align, 2);
    append(&bits, 2);
    append("data", 4);
    append(&data_bytes, 4);
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        const auto sample = static_cast<std::int16_t>(
            std::sin(2.0 * 3.14159265358979323846 * 1'000.0 * frame / sample_rate) * 12'000.0
        );
        append(&sample, sizeof(sample));
    }
    return wav;
}

} // namespace

int main() {
    const std::filesystem::path path =
        std::filesystem::temp_directory_path()
        / ("echo-spectrogram-test-"
           + std::to_string(std::chrono::system_clock::now().time_since_epoch().count()) + ".wav");
    {
        std::ofstream file(path, std::ios::binary);
        const std::string wav = sine_wav();
        file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }
    const auto overview = echo::audio::build_spectrogram_overview(path.string(), 32, 64);
    std::remove(path.c_str());
    if (overview.canonical_sample_rate != 48'000 || overview.window_frames != 2'048
        || overview.hop_frames != 512 || overview.time_columns == 0 || overview.time_columns > 32
        || overview.frequency_bins != 64
        || overview.magnitudes.size()
               != static_cast<std::size_t>(overview.time_columns * overview.frequency_bins)) {
        return 1;
    }
    const std::size_t expected_row = 1'000U * overview.frequency_bins / 24'000U;
    std::uint8_t tone = 0;
    std::uint8_t edge = 0;
    for (std::size_t column = 0; column < overview.time_columns; ++column) {
        tone = std::max(tone, overview.magnitudes[column * overview.frequency_bins + expected_row]);
        edge = std::max(edge, overview.magnitudes[column * overview.frequency_bins]);
    }
    return tone > edge ? 0 : 1;
}
