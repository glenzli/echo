#include "echo/audio/convolution_space_processor.hpp"
#include "echo/audio/impulse_response_preparer.hpp"
#include "echo/audio/prepared_impulse_response.hpp"

#include <bit>
#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

void append_u16(std::string& bytes, std::uint16_t value) {
    bytes.push_back(static_cast<char>(value & 0xffU));
    bytes.push_back(static_cast<char>((value >> 8U) & 0xffU));
}

void append_u32(std::string& bytes, std::uint32_t value) {
    bytes.push_back(static_cast<char>(value & 0xffU));
    bytes.push_back(static_cast<char>((value >> 8U) & 0xffU));
    bytes.push_back(static_cast<char>((value >> 16U) & 0xffU));
    bytes.push_back(static_cast<char>((value >> 24U) & 0xffU));
}

std::string stereo_wav() {
    constexpr std::uint32_t sample_rate = 48000;
    constexpr std::uint16_t channels = 2;
    constexpr std::uint16_t frames = 4;
    constexpr std::uint32_t data_bytes = frames * channels * 2U;
    std::string wav;
    wav.append("RIFF", 4);
    append_u32(wav, 36U + data_bytes);
    wav.append("WAVEfmt ", 8);
    append_u32(wav, 16);
    append_u16(wav, 1);
    append_u16(wav, channels);
    append_u32(wav, sample_rate);
    append_u32(wav, sample_rate * channels * 2U);
    append_u16(wav, channels * 2U);
    append_u16(wav, 16);
    wav.append("data", 4);
    append_u32(wav, data_bytes);
    for (std::uint16_t frame = 0; frame < frames; ++frame) {
        append_u16(wav, frame == 0U ? 24576U : 0U);
        append_u16(wav, frame == 1U ? static_cast<std::uint16_t>(-16384) : 0U);
    }
    return wav;
}

std::filesystem::path unique_path(const std::string& role) {
    const auto suffix = std::to_string(std::chrono::steady_clock::now().time_since_epoch().count());
    return std::filesystem::temp_directory_path() / ("echo-prepared-ir-" + role + suffix);
}

void write_file(const std::filesystem::path& path, const std::string& bytes) {
    std::ofstream output(path, std::ios::binary);
    output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
    assert(output.good());
}

std::string read_file(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    return std::string(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
}

void expect_rejected(const std::filesystem::path& path) {
    bool rejected = false;
    try {
        static_cast<void>(echo::audio::load_prepared_impulse_response(path.string()));
    } catch (const std::runtime_error&) {
        rejected = true;
    }
    assert(rejected);
}

} // namespace

int main() {
    const auto source = unique_path("source.wav");
    const auto prepared = unique_path("prepared.echoir");
    write_file(source, stereo_wav());
    const auto preparation =
        echo::audio::prepare_impulse_response(source.string(), prepared.string());
    const auto loaded = echo::audio::load_prepared_impulse_response(prepared.string());
    assert(loaded.preparation_version == preparation.preparation_version);
    assert(loaded.source_sample_rate == 48000);
    assert(loaded.source_frame_count == 4);
    assert(loaded.avcodec_version == preparation.avcodec_version);
    assert(loaded.swresample_version == preparation.swresample_version);
    assert(loaded.left.size() == 4);
    assert(loaded.right.size() == 4);
    assert(std::abs(loaded.left[0] - 0.75F) < 0.00001F);
    assert(std::abs(loaded.right[1] + 0.5F) < 0.00001F);

    echo::audio::ConvolutionSpaceProcessor processor(
        {.enabled = true, .mix_percent = 100, .wet_gain_centibels = 0},
        48000,
        2,
        loaded.left,
        loaded.right
    );
    std::vector<float> program(16, 0.0F);
    program[0] = 1.0F;
    program[1] = 1.0F;
    processor.process_interleaved(program.data(), 8, 2);
    assert(std::abs(program[0] - 0.75F) < 0.00002F);
    assert(std::abs(program[1]) < 0.00002F);
    assert(std::abs(program[2]) < 0.00002F);
    assert(std::abs(program[3] + 0.5F) < 0.00002F);

    const std::string valid = read_file(prepared);
    auto unknown_version = valid;
    unknown_version[12] = 2;
    const auto unknown_path = unique_path("unknown.echoir");
    write_file(unknown_path, unknown_version);
    expect_rejected(unknown_path);

    auto nonfinite = valid;
    const std::uint32_t nan_bits =
        std::bit_cast<std::uint32_t>(std::numeric_limits<float>::quiet_NaN());
    for (std::size_t byte = 0; byte < sizeof(float); ++byte) {
        nonfinite[64U + byte] = static_cast<char>((nan_bits >> (byte * 8U)) & 0xffU);
    }
    const auto nonfinite_path = unique_path("nonfinite.echoir");
    write_file(nonfinite_path, nonfinite);
    expect_rejected(nonfinite_path);

    const auto truncated_path = unique_path("truncated.echoir");
    write_file(truncated_path, valid.substr(0, valid.size() - 1U));
    expect_rejected(truncated_path);

    std::filesystem::remove(source);
    std::filesystem::remove(prepared);
    std::filesystem::remove(unknown_path);
    std::filesystem::remove(nonfinite_path);
    std::filesystem::remove(truncated_path);
}
