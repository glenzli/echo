#include "echo/audio/spectrogram_detail.hpp"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <numbers>
#include <stdexcept>

namespace {
void require(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
std::filesystem::path fixture() {
    const auto path =
        std::filesystem::temp_directory_path()
        / ("echo-spectrum-detail-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".wav");
    std::ofstream file(path, std::ios::binary);
    const auto write = [&](auto value) {
        file.write(reinterpret_cast<const char*>(&value), sizeof(value));
    };
    constexpr std::uint32_t frames = 48'000 * 8, bytes = frames * 4;
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
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        const double time = static_cast<double>(frame) / 48'000;
        const double tone = (time >= 1 && time < 2 ? 700 : (time >= 5 && time < 6 ? 3000 : 0));
        const double signal =
            0.22 * std::sin(2 * std::numbers::pi * 100 * time)
            + (tone > 0 ? 0.18 * std::sin(2 * std::numbers::pi * tone * time) : 0);
        const auto sample = static_cast<std::int16_t>(signal * 32767);
        write(sample);
        write(static_cast<std::int16_t>(-sample));
    }
    return path;
}
std::uint8_t level(
    const echo::audio::SpectrogramDetail& detail,
    const echo::audio::SpectrogramDetailRequest& request,
    double millis,
    double hz
) {
    const auto x = static_cast<std::size_t>(
        (millis - request.start_millis) / (request.end_millis - request.start_millis)
        * detail.columns
    );
    const auto y = static_cast<std::size_t>(
        std::log(hz / request.low_hertz) / std::log(request.high_hertz / request.low_hertz)
        * detail.rows
    );
    std::uint8_t maximum = 0;
    for (auto row = y > 0 ? y - 1 : 0; row < std::min<std::size_t>(detail.rows, y + 2); ++row)
        maximum = std::max(maximum, detail.magnitudes[x * detail.rows + row]);
    return maximum;
}
} // namespace
int main() {
    const auto path = fixture();
    try {
        echo::audio::SpectrogramDetailRequest request{
            .start_millis = 0,
            .end_millis = 8000,
            .low_hertz = 20,
            .high_hertz = 8000,
            .columns = 160,
            .rows = 256
        };
        const auto detail = echo::audio::build_spectrogram_detail(path.string(), request);
        require(detail.magnitudes.size() == 160 * 256, "unbounded image");
        require(level(detail, request, 1500, 700) > 170, "early tone lost or time misplaced");
        require(level(detail, request, 5500, 3000) > 170, "late tone lost or time misplaced");
        require(level(detail, request, 1500, 3000) < 60, "late event leaked to early bucket");
        require(level(detail, request, 5500, 700) < 60, "early event leaked to late bucket");
        require(level(detail, request, 3500, 100) > 170, "out-of-phase stereo disappeared");
        request.start_millis = 5000;
        request.end_millis = 6000;
        request.low_hertz = 2000;
        request.high_hertz = 4000;
        const auto zoom = echo::audio::build_spectrogram_detail(path.string(), request);
        require(
            level(zoom, request, 5500, 3000) > 170,
            "viewport failed to decode at Original time"
        );
        request.start_millis = 0;
        request.end_millis = 10;
        const auto short_view = echo::audio::build_spectrogram_detail(path.string(), request);
        require(short_view.magnitudes.size() == 160 * 256, "short viewport failed");
        std::stop_source cancel;
        cancel.request_stop();
        bool rejected = false;
        try {
            (void)echo::audio::build_spectrogram_detail(path.string(), request, cancel.get_token());
        } catch (const std::runtime_error&) {
            rejected = true;
        }
        require(rejected, "cancelled viewport was published");
        std::filesystem::remove(path);
        std::cout << "spectrogram detail contracts passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::filesystem::remove(path);
        std::cerr << error.what() << '\n';
        return 1;
    }
}
