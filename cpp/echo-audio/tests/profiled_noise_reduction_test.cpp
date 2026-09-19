#include "echo/audio/playback.hpp"
#include "echo/audio/profiled_noise_reduction.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <numbers>
#include <random>
#include <stdexcept>
#include <thread>

namespace {
void require(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}

std::filesystem::path fixture() {
    const auto path =
        std::filesystem::temp_directory_path()
        / ("echo-noise-profile-"
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

double rms(const std::vector<float>& value, std::size_t first, std::size_t last) {
    double sum = 0;
    for (auto frame = first; frame < last; ++frame)
        sum += value[frame * 2] * value[frame * 2];
    return std::sqrt(sum / static_cast<double>(last - first));
}
double tone(const std::vector<float>& value) {
    double re = 0, im = 0;
    for (std::size_t frame = 3 * 48'000; frame < 4 * 48'000; ++frame) {
        const double angle = 2 * std::numbers::pi * 1000 * static_cast<double>(frame) / 48'000;
        re += value[frame * 2] * std::cos(angle);
        im += value[frame * 2] * std::sin(angle);
    }
    return 2 * std::hypot(re, im) / 48'000;
}
} // namespace

int main() {
    const auto path = fixture();
    try {
        auto profile = echo::audio::learn_noise_profile(path.string(), 100, 900);
        require(
            !profile.enabled && profile.power_centibels.size() == 1025,
            "capture must preserve a disabled complete profile"
        );
        auto source = render(path.string(), {});
        echo::audio::PlaybackAdjustment adjustment;
        adjustment.profiled_noise_reduction = profile;
        require(source == render(path.string(), adjustment), "bypass changed the source");
        profile.enabled = true;
        profile.reduction_centibels = 1800;
        adjustment.profiled_noise_reduction = profile;
        const auto reduced = render(path.string(), adjustment);
        profile.residue = true;
        adjustment.profiled_noise_reduction = profile;
        const auto removed = render(path.string(), adjustment);
        require(
            source.size() == reduced.size() && source.size() == removed.size(),
            "noise reduction changed duration"
        );
        const double reduction =
            20 * std::log10(rms(reduced, 48'000, 90'000) / rms(source, 48'000, 90'000));
        const double retained = 20 * std::log10(tone(reduced) / tone(source));
        std::cout << "noise reduction dB=" << reduction << ", retained tone dB=" << retained
                  << '\n';
        require(reduction < -10, "steady noise was not reduced");
        require(std::abs(retained) < 0.5, "wanted signal was excessively attenuated");
        float input_peak = 0, output_peak = 0;
        for (std::size_t sample = 0; sample < 48'000 * 2; ++sample) {
            input_peak = std::max(input_peak, std::abs(source[sample]));
            output_peak = std::max(output_peak, std::abs(reduced[sample]));
        }
        std::cout << "initial peaks: " << input_peak << " -> " << output_peak << '\n';
        require(
            output_peak < input_peak * 1.5F,
            "spectral startup amplified noise at the boundary"
        );
        for (std::size_t sample = 0; sample < source.size(); ++sample) {
            require(std::isfinite(reduced[sample]), "non-finite output");
            require(
                std::abs(reduced[sample] + removed[sample] - source[sample]) < 0.00002F,
                "residue did not complement reduced audio"
            );
        }
        for (std::size_t frame = 0; frame < source.size() / 2; ++frame)
            require(
                std::abs(reduced[frame * 2] + reduced[frame * 2 + 1]) < 0.000002F,
                "stereo phase relationship changed"
            );
        profile.power_centibels.pop_back();
        bool rejected = false;
        try {
            echo::audio::validate_noise_profile(profile, 5000);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        require(rejected, "incomplete profile was accepted");
        std::stop_source cancellation;
        cancellation.request_stop();
        rejected = false;
        try {
            (void)
                echo::audio::learn_noise_profile(path.string(), 100, 900, cancellation.get_token());
        } catch (const std::runtime_error&) {
            rejected = true;
        }
        require(rejected, "cancelled capture returned a profile");
        std::filesystem::remove(path);
        return 0;
    } catch (const std::exception& error) {
        std::filesystem::remove(path);
        std::cerr << error.what() << '\n';
        return 1;
    }
}
