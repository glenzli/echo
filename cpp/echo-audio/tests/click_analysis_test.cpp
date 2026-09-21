#include "echo/audio/click_analysis.hpp"
#include <algorithm>
#include <cassert>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <limits>
#include <stdexcept>

using namespace echo::audio;
namespace {
constexpr DeClickParameters parameters{.enabled = true, .sensitivity_percent = 85};
std::vector<float> tone(std::size_t frames) {
    std::vector<float> values(frames * 2);
    for (std::size_t i = 0; i < frames; ++i) {
        values[i * 2] =
            0.15F * std::sin(static_cast<float>(i) * 6.28318530718F * 220.0F / 48000.0F);
        values[i * 2 + 1] =
            0.12F * std::sin(static_cast<float>(i) * 6.28318530718F * 733.0F / 48000.0F);
    }
    return values;
}
ClickAnalysisResult analyze(const std::vector<float>& values, std::size_t block) {
    ClickCandidateAnalyzer scanner(parameters, 2);
    for (std::size_t i = 0; i < values.size() / 2; i += block)
        scanner.process_interleaved(values.data() + i * 2, std::min(block, values.size() / 2 - i));
    return scanner.result();
}
struct Wave {
    std::filesystem::path path =
        std::filesystem::temp_directory_path()
        / ("echo-click-test-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".wav");
    explicit Wave(const std::vector<float>& values) {
        std::ofstream file(path, std::ios::binary);
        const auto write = [&](auto value) {
            file.write(reinterpret_cast<const char*>(&value), sizeof(value));
        };
        const auto bytes = static_cast<std::uint32_t>(values.size() * 2);
        file.write("RIFF", 4);
        write(bytes + 36);
        file.write("WAVEfmt ", 8);
        write(std::uint32_t{16});
        write(std::uint16_t{1});
        write(std::uint16_t{2});
        write(std::uint32_t{48000});
        write(std::uint32_t{192000});
        write(std::uint16_t{4});
        write(std::uint16_t{16});
        file.write("data", 4);
        write(bytes);
        for (float value : values)
            write(static_cast<std::int16_t>(value * 32767.0F));
    }
    ~Wave() {
        std::filesystem::remove(path);
    }
};
} // namespace
int main() {
    auto values = tone(48000);
    assert(analyze(values, 137).total_candidates == 0);
    values[4095 * 2] += 0.75F;
    values[15000 * 2 + 1] -= 0.7F;
    const auto whole = analyze(values, 48000);
    assert(whole.total_candidates == 2 && whole.candidates.size() == 2);
    assert(
        whole.candidates[0].start_frame <= 4095 && whole.candidates[0].end_frame > 4095
        && whole.candidates[0].channel_mask == 1
    );
    assert(whole.candidates[1].channel_mask == 2);
    for (const std::size_t block : {1, 127, 4096}) {
        const auto other = analyze(values, block);
        assert(other.total_candidates == whole.total_candidates);
        for (std::size_t i = 0; i < whole.candidates.size(); ++i) {
            assert(other.candidates[i].start_frame == whole.candidates[i].start_frame);
            assert(other.candidates[i].end_frame == whole.candidates[i].end_frame);
            assert(
                other.candidates[i].maximum_difference == whole.candidates[i].maximum_difference
            );
        }
    }
    std::vector<float> busy(2 * 100000, 0);
    for (std::size_t i = 0; i < 300; ++i)
        busy[(1000 + i * 240) * 2] = 0.8F;
    const auto capped = analyze(busy, 4096);
    assert(capped.total_candidates == 300 && capped.candidates.size() == 256);
    ClickCandidateAnalyzer window(parameters, 2, 70000, 80000);
    window.process_interleaved(busy.data(), 100000);
    const auto later = window.result();
    assert(later.total_candidates > 0 && later.total_candidates < 20);
    assert(later.candidates.front().start_frame >= 70000);
    Wave wave(values);
    auto file = analyze_clicks(wave.path.string(), 300, 340, parameters);
    assert(
        file.total_candidates == 1 && file.candidates[0].start_frame <= 15000
        && file.candidates[0].end_frame > 15000
    );
    // A finding immediately before the selected interval is not returned.
    file = analyze_clicks(wave.path.string(), 320, 400, parameters);
    assert(file.total_candidates == 0);
    std::stop_source cancelled;
    cancelled.request_stop();
    bool rejected = false;
    try {
        (void)analyze_clicks(wave.path.string(), 0, 100, parameters, cancelled.get_token());
    } catch (const std::runtime_error&) {
        rejected = true;
    }
    assert(rejected);
    rejected = false;
    try {
        (void)analyze_clicks(wave.path.string(), 0, 300001, parameters);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
    rejected = false;
    try {
        (void)analyze_clicks(wave.path.string(), 0, 1001, parameters);
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    assert(rejected);
    values[0] = std::numeric_limits<float>::quiet_NaN();
    rejected = false;
    try {
        (void)analyze(values, 1);
    } catch (const std::runtime_error&) {
        rejected = true;
    }
    assert(rejected);
}
