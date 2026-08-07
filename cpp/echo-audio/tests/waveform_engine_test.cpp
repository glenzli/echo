//! Focused engine test: streaming decode plus waveform pyramid over a
//! synthesized WAV. The WAV is generated in-memory, so no fixture corpus is
//! required.

#include <echo/audio/waveform.hpp>

#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

namespace {

std::string synthesize_sine_wav(std::uint32_t sample_rate, double seconds) {
    const std::uint16_t channels = 1;
    const std::uint16_t bits = 16;
    const std::uint32_t sample_count =
        static_cast<std::uint32_t>(static_cast<double>(sample_rate) * seconds);
    const std::uint32_t data_bytes = sample_count * channels * bits / 8;

    std::string wav;
    wav.reserve(44 + data_bytes);
    const auto append = [&wav](const void* bytes, std::size_t size) {
        wav.append(static_cast<const char*>(bytes), size);
    };
    append("RIFF", 4);
    const std::uint32_t riff_size = 36 + data_bytes;
    append(&riff_size, 4);
    append("WAVE", 4);
    append("fmt ", 4);
    const std::uint32_t fmt_size = 16;
    append(&fmt_size, 4);
    const std::uint16_t format = 1;
    append(&format, 2);
    append(&channels, 2);
    append(&sample_rate, 4);
    const std::uint32_t byte_rate = sample_rate * channels * bits / 8;
    append(&byte_rate, 4);
    const std::uint16_t block_align = channels * bits / 8;
    append(&block_align, 2);
    append(&bits, 2);
    append("data", 4);
    append(&data_bytes, 4);
    for (std::uint32_t index = 0; index < sample_count; ++index) {
        const double phase = 2.0 * 3.14159265358979323846 * 440.0 * static_cast<double>(index)
                             / static_cast<double>(sample_rate);
        const std::int16_t sample = static_cast<std::int16_t>(std::sin(phase) * 12000.0);
        append(&sample, 2);
    }
    return wav;
}

int failures = 0;

void expect(bool condition, const char* message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        ++failures;
    }
}

} // namespace

int main() {
    const std::filesystem::path path =
        std::filesystem::temp_directory_path()
        / ("echo-waveform-test-"
           + std::to_string(std::chrono::system_clock::now().time_since_epoch().count()) + ".wav");
    {
        std::ofstream file(path, std::ios::binary);
        const std::string wav = synthesize_sine_wav(44100, 1.0);
        file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }

    const echo::audio::Waveform waveform = echo::audio::build_waveform(path, 8);
    std::remove(path.c_str());

    expect(waveform.canonical_sample_rate == 48000, "canonical rate is 48 kHz");
    // Height is bounded by content: ~100 base buckets halve 6 times, so the
    // pyramid lands at 7 levels, not the requested 8.
    expect(waveform.levels.size() >= 2, "pyramid has more than the base level");
    const auto& base = waveform.levels.front();
    // 1 s at 48 kHz with 480-sample buckets is ~100 buckets; WAV decode
    // should land inside a tight tolerance.
    expect(base.mins.size() >= 90 && base.mins.size() <= 110, "base bucket count");
    expect(base.mins.size() == base.maxs.size(), "min/max pairs stay aligned");
    for (std::size_t index = 0; index < base.mins.size(); ++index) {
        expect(base.mins[index] <= base.maxs[index], "min never exceeds max");
    }
    for (std::size_t level = 1; level < waveform.levels.size(); ++level) {
        expect(
            waveform.levels[level].mins.size() < waveform.levels[level - 1].mins.size(),
            "each level strictly coarsens"
        );
    }

    if (failures == 0) {
        std::printf("waveform engine test: ok (%zu base buckets)\n", base.mins.size());
        return 0;
    }
    return 1;
}
