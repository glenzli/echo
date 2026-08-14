#include "echo/audio/impulse_response_preparer.hpp"

#include <algorithm>
#include <bit>
#include <cassert>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

void append_u16(std::string& target, std::uint16_t value) {
    target.push_back(static_cast<char>(value & 0xffU));
    target.push_back(static_cast<char>((value >> 8U) & 0xffU));
}

void append_u24(std::string& target, std::int32_t value) {
    const auto bits = static_cast<std::uint32_t>(value);
    target.push_back(static_cast<char>(bits & 0xffU));
    target.push_back(static_cast<char>((bits >> 8U) & 0xffU));
    target.push_back(static_cast<char>((bits >> 16U) & 0xffU));
}

void append_u32(std::string& target, std::uint32_t value) {
    target.push_back(static_cast<char>(value & 0xffU));
    target.push_back(static_cast<char>((value >> 8U) & 0xffU));
    target.push_back(static_cast<char>((value >> 16U) & 0xffU));
    target.push_back(static_cast<char>((value >> 24U) & 0xffU));
}

std::uint32_t read_u32(const std::string& bytes, std::size_t offset) {
    const auto byte = [&](std::size_t index) {
        return static_cast<std::uint32_t>(static_cast<unsigned char>(bytes[offset + index]));
    };
    return byte(0) | (byte(1) << 8U) | (byte(2) << 16U) | (byte(3) << 24U);
}

std::uint64_t read_u64(const std::string& bytes, std::size_t offset) {
    return read_u32(bytes, offset)
           | (static_cast<std::uint64_t>(read_u32(bytes, offset + 4)) << 32U);
}

float read_float(const std::string& bytes, std::size_t offset) {
    return std::bit_cast<float>(read_u32(bytes, offset));
}

std::string classic_pcm16(
    std::uint32_t sample_rate,
    std::uint16_t channel_count,
    const std::vector<std::int16_t>& samples,
    std::uint16_t format_tag = 1
) {
    const std::uint32_t data_bytes = static_cast<std::uint32_t>(samples.size() * 2U);
    std::string wav;
    wav.append("RIFF", 4);
    append_u32(wav, 36U + data_bytes);
    wav.append("WAVEfmt ", 8);
    append_u32(wav, 16);
    append_u16(wav, format_tag);
    append_u16(wav, channel_count);
    append_u32(wav, sample_rate);
    append_u32(wav, sample_rate * channel_count * 2U);
    append_u16(wav, static_cast<std::uint16_t>(channel_count * 2U));
    append_u16(wav, 16);
    wav.append("data", 4);
    append_u32(wav, data_bytes);
    for (const std::int16_t sample : samples) {
        append_u16(wav, static_cast<std::uint16_t>(sample));
    }
    return wav;
}

std::string classic_float32(std::uint32_t sample_rate, const std::vector<float>& samples) {
    const std::uint32_t data_bytes = static_cast<std::uint32_t>(samples.size() * 4U);
    std::string wav;
    wav.append("RIFF", 4);
    append_u32(wav, 36U + data_bytes);
    wav.append("WAVEfmt ", 8);
    append_u32(wav, 16);
    append_u16(wav, 3);
    append_u16(wav, 1);
    append_u32(wav, sample_rate);
    append_u32(wav, sample_rate * 4U);
    append_u16(wav, 4);
    append_u16(wav, 32);
    wav.append("data", 4);
    append_u32(wav, data_bytes);
    for (const float sample : samples) {
        append_u32(wav, std::bit_cast<std::uint32_t>(sample));
    }
    return wav;
}

std::string classic_pcm32(std::uint32_t sample_rate, const std::vector<std::int32_t>& samples) {
    const std::uint32_t data_bytes = static_cast<std::uint32_t>(samples.size() * 4U);
    std::string wav;
    wav.append("RIFF", 4);
    append_u32(wav, 36U + data_bytes);
    wav.append("WAVEfmt ", 8);
    append_u32(wav, 16);
    append_u16(wav, 1);
    append_u16(wav, 1);
    append_u32(wav, sample_rate);
    append_u32(wav, sample_rate * 4U);
    append_u16(wav, 4);
    append_u16(wav, 32);
    wav.append("data", 4);
    append_u32(wav, data_bytes);
    for (const std::int32_t sample : samples) {
        append_u32(wav, static_cast<std::uint32_t>(sample));
    }
    return wav;
}

std::string extensible_pcm24(std::uint32_t channel_mask) {
    constexpr std::uint32_t sample_rate = 44100;
    constexpr std::uint16_t channels = 2;
    constexpr std::uint32_t frames = 441;
    constexpr std::uint32_t data_bytes = frames * channels * 3U;
    std::string wav;
    wav.append("RIFF", 4);
    append_u32(wav, 60U + data_bytes);
    wav.append("WAVEfmt ", 8);
    append_u32(wav, 40);
    append_u16(wav, 0xfffe);
    append_u16(wav, channels);
    append_u32(wav, sample_rate);
    append_u32(wav, sample_rate * channels * 3U);
    append_u16(wav, channels * 3U);
    append_u16(wav, 24);
    append_u16(wav, 22);
    append_u16(wav, 24);
    append_u32(wav, channel_mask);
    append_u32(wav, 1);
    append_u16(wav, 0);
    append_u16(wav, 0x0010);
    wav.push_back(static_cast<char>(0x80));
    wav.push_back(0);
    wav.push_back(0);
    wav.push_back(static_cast<char>(0xaa));
    wav.push_back(0);
    wav.push_back(static_cast<char>(0x38));
    wav.push_back(static_cast<char>(0x9b));
    wav.push_back(static_cast<char>(0x71));
    wav.append("data", 4);
    append_u32(wav, data_bytes);
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        append_u24(wav, frame == 0U ? 0x600000 : 0);
        append_u24(wav, frame == 220U ? -0x500000 : 0);
    }
    return wav;
}

std::string extensible_true_stereo_pcm24(std::uint32_t channel_mask) {
    constexpr std::uint32_t sample_rate = 48000;
    constexpr std::uint16_t channels = 4;
    constexpr std::uint32_t frames = 4;
    constexpr std::uint32_t data_bytes = frames * channels * 3U;
    std::string wav;
    wav.append("RIFF", 4);
    append_u32(wav, 60U + data_bytes);
    wav.append("WAVEfmt ", 8);
    append_u32(wav, 40);
    append_u16(wav, 0xfffe);
    append_u16(wav, channels);
    append_u32(wav, sample_rate);
    append_u32(wav, sample_rate * channels * 3U);
    append_u16(wav, channels * 3U);
    append_u16(wav, 24);
    append_u16(wav, 22);
    append_u16(wav, 24);
    append_u32(wav, channel_mask);
    append_u32(wav, 1);
    append_u16(wav, 0);
    append_u16(wav, 0x0010);
    wav.push_back(static_cast<char>(0x80));
    wav.push_back(0);
    wav.push_back(0);
    wav.push_back(static_cast<char>(0xaa));
    wav.push_back(0);
    wav.push_back(static_cast<char>(0x38));
    wav.push_back(static_cast<char>(0x9b));
    wav.push_back(static_cast<char>(0x71));
    wav.append("data", 4);
    append_u32(wav, data_bytes);
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        append_u24(wav, frame == 0U ? 0x400000 : 0);  // LL
        append_u24(wav, frame == 1U ? 0x300000 : 0);  // LR
        append_u24(wav, frame == 2U ? -0x400000 : 0); // RL
        append_u24(wav, frame == 3U ? -0x200000 : 0); // RR
    }
    return wav;
}

std::filesystem::path unique_path(const std::string& role) {
    const auto suffix = std::to_string(std::chrono::steady_clock::now().time_since_epoch().count());
    return std::filesystem::temp_directory_path() / ("echo-ir-" + role + "-" + suffix);
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

void expect_rejected(
    const std::string& wav,
    const std::string& role,
    echo::audio::ImpulseResponsePreparationLayout layout =
        echo::audio::ImpulseResponsePreparationLayout::AutoMonoOrStereo
) {
    const auto source = unique_path(role + ".wav");
    const auto output = unique_path(role + ".echoir");
    write_file(source, wav);
    bool rejected = false;
    try {
        static_cast<void>(
            echo::audio::prepare_impulse_response(source.string(), output.string(), layout)
        );
    } catch (const std::runtime_error&) {
        rejected = true;
    }
    assert(rejected);
    std::filesystem::remove(source);
    std::filesystem::remove(output);
}

void canonical_mono_resampling_and_header_are_stable() {
    constexpr std::uint32_t source_rate = 24000;
    std::vector<std::int16_t> samples(source_rate / 10U, 0);
    samples[0] = 30000;
    samples[333] = -12000;
    const auto source = unique_path("mono.wav");
    const auto output = unique_path("mono.echoir");
    const auto repeated_output = unique_path("mono-repeated.echoir");
    write_file(source, classic_pcm16(source_rate, 1, samples));

    const auto result = echo::audio::prepare_impulse_response(source.string(), output.string());
    const std::string prepared = read_file(output);
    const auto repeated =
        echo::audio::prepare_impulse_response(source.string(), repeated_output.string());
    assert(repeated == result);
    assert(read_file(repeated_output) == prepared);
    assert(prepared.substr(0, 8) == "ECHOIR01");
    assert(result.preparation_version == 1);
    assert(result.source_sample_rate == source_rate);
    assert(result.channel_count == 1);
    assert(result.source_frame_count == samples.size());
    assert(result.prepared_frame_count == 4800);
    assert(result.size_bytes == prepared.size());
    assert(read_u32(prepared, 8) == 64);
    assert(read_u32(prepared, 12) == result.preparation_version);
    assert(read_u32(prepared, 16) == 48000);
    assert(read_u32(prepared, 20) == 1);
    assert(read_u64(prepared, 24) == result.prepared_frame_count);
    assert(read_u32(prepared, 32) == source_rate);
    assert(read_u64(prepared, 40) == result.source_frame_count);
    assert(read_u32(prepared, 48) == result.avcodec_version);
    assert(read_u32(prepared, 52) == result.swresample_version);
    assert(read_u64(prepared, 56) == result.prepared_frame_count * sizeof(float));
    bool has_nonzero = false;
    for (std::uint64_t frame = 0; frame < result.prepared_frame_count; ++frame) {
        const float sample = read_float(prepared, 64U + frame * sizeof(float));
        assert(std::isfinite(sample));
        has_nonzero = has_nonzero || sample != 0.0F;
    }
    assert(has_nonzero);
    std::filesystem::remove(source);
    std::filesystem::remove(output);
    std::filesystem::remove(repeated_output);
}

void stereo_planar_routing_and_extensible_pcm24_are_preserved() {
    const auto source = unique_path("stereo.wav");
    const auto output = unique_path("stereo.echoir");
    write_file(source, extensible_pcm24(0x00000003));
    const auto result = echo::audio::prepare_impulse_response(source.string(), output.string());
    const std::string prepared = read_file(output);
    assert(result.source_sample_rate == 44100);
    assert(result.channel_count == 2);
    assert(result.source_frame_count == 441);
    assert(result.prepared_frame_count == 480);
    const std::size_t right_offset =
        64U + static_cast<std::size_t>(result.prepared_frame_count) * sizeof(float);
    std::size_t left_peak = 0;
    std::size_t right_peak = 0;
    float left_value = 0.0F;
    float right_value = 0.0F;
    for (std::size_t frame = 0; frame < result.prepared_frame_count; ++frame) {
        const float left = std::abs(read_float(prepared, 64U + frame * sizeof(float)));
        const float right = std::abs(read_float(prepared, right_offset + frame * sizeof(float)));
        if (left > left_value) {
            left_value = left;
            left_peak = frame;
        }
        if (right > right_value) {
            right_value = right;
            right_peak = frame;
        }
    }
    assert(left_peak < 8U);
    assert(right_peak > 230U && right_peak < 250U);
    assert(left_value > 0.5F);
    assert(right_value > 0.4F);
    std::filesystem::remove(source);
    std::filesystem::remove(output);
}

void explicit_true_stereo_layout_writes_stable_v2_ll_lr_rl_rr_planes() {
    using echo::audio::ImpulseResponsePreparationLayout;
    const auto source = unique_path("true-stereo.wav");
    const auto output = unique_path("true-stereo.echoir");
    const auto repeated_output = unique_path("true-stereo-repeated.echoir");
    write_file(source, extensible_true_stereo_pcm24(0x00000033));
    const auto result = echo::audio::prepare_impulse_response(
        source.string(),
        output.string(),
        ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr
    );
    const auto repeated = echo::audio::prepare_impulse_response(
        source.string(),
        repeated_output.string(),
        ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr
    );
    const std::string prepared = read_file(output);
    assert(result == repeated);
    assert(read_file(repeated_output) == prepared);
    assert(result.preparation_version == 2);
    assert(result.source_sample_rate == 48000);
    assert(result.channel_count == 4);
    assert(result.source_frame_count == 4);
    assert(result.prepared_frame_count == 4);
    assert(read_u32(prepared, 8) == 64);
    assert(read_u32(prepared, 12) == 2);
    assert(read_u32(prepared, 20) == 4);
    assert(read_u64(prepared, 56) == 4U * 4U * sizeof(float));
    const std::size_t plane_bytes = 4U * sizeof(float);
    assert(std::abs(read_float(prepared, 64) - 0.5F) < 0.000001F);
    assert(std::abs(read_float(prepared, 64 + plane_bytes + sizeof(float)) - 0.375F) < 0.000001F);
    assert(
        std::abs(read_float(prepared, 64 + 2U * plane_bytes + 2U * sizeof(float)) + 0.5F)
        < 0.000001F
    );
    assert(
        std::abs(read_float(prepared, 64 + 3U * plane_bytes + 3U * sizeof(float)) + 0.25F)
        < 0.000001F
    );
    std::filesystem::remove(source);
    std::filesystem::remove(output);
    std::filesystem::remove(repeated_output);
}

void invalid_containers_formats_layouts_and_samples_fail_closed() {
    using echo::audio::ImpulseResponsePreparationLayout;
    std::string rf64 = classic_pcm16(48000, 1, {1000});
    rf64.replace(0, 4, "RF64");
    expect_rejected(rf64, "rf64");
    expect_rejected(classic_pcm16(48000, 1, {1000}, 6), "compressed");
    expect_rejected(classic_pcm16(48000, 3, {1000, 0, 0}), "three-channel");
    expect_rejected(extensible_pcm24(0), "ambiguous-mask");
    expect_rejected(extensible_true_stereo_pcm24(0x00000033), "implicit-true-stereo");
    expect_rejected(
        extensible_true_stereo_pcm24(0),
        "true-stereo-ambiguous-mask",
        ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr
    );
    expect_rejected(
        classic_pcm16(48000, 4, {1000, 0, 0, 0}),
        "classic-true-stereo",
        ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr
    );
    expect_rejected(
        extensible_pcm24(0x00000003),
        "stereo-as-true-stereo",
        ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr
    );
    expect_rejected(classic_pcm16(48000, 1, std::vector<std::int16_t>(128, 0)), "silence");
    expect_rejected(
        classic_float32(48000, {0.5F, std::numeric_limits<float>::quiet_NaN()}),
        "nonfinite"
    );
}

void accepted_pcm32_and_float32_decode_to_finite_canonical_samples() {
    const auto pcm_source = unique_path("pcm32.wav");
    const auto pcm_output = unique_path("pcm32.echoir");
    write_file(pcm_source, classic_pcm32(48000, {0x60000000, -0x40000000, 0, 0}));
    const auto pcm =
        echo::audio::prepare_impulse_response(pcm_source.string(), pcm_output.string());
    assert(pcm.channel_count == 1);
    assert(pcm.prepared_frame_count == 4);
    const std::string pcm_prepared = read_file(pcm_output);
    assert(std::abs(read_float(pcm_prepared, 64) - 0.75F) < 0.000001F);
    assert(std::abs(read_float(pcm_prepared, 68) + 0.5F) < 0.000001F);

    const auto float_source = unique_path("float.wav");
    const auto float_output = unique_path("float.echoir");
    write_file(float_source, classic_float32(48000, {0.25F, -0.75F, 0.5F, 0.0F}));
    const auto floating =
        echo::audio::prepare_impulse_response(float_source.string(), float_output.string());
    assert(floating.channel_count == 1);
    assert(floating.prepared_frame_count == 4);
    const std::string float_prepared = read_file(float_output);
    assert(read_float(float_prepared, 64) == 0.25F);
    assert(read_float(float_prepared, 68) == -0.75F);

    std::filesystem::remove(pcm_source);
    std::filesystem::remove(pcm_output);
    std::filesystem::remove(float_source);
    std::filesystem::remove(float_output);
}

void prepared_duration_is_bounded() {
    constexpr std::uint32_t sample_rate = 8000;
    std::vector<std::int16_t> samples(sample_rate * 5U + 1U, 1);
    expect_rejected(classic_pcm16(sample_rate, 1, samples), "too-long");
}

} // namespace

int main() {
    canonical_mono_resampling_and_header_are_stable();
    stereo_planar_routing_and_extensible_pcm24_are_preserved();
    explicit_true_stereo_layout_writes_stable_v2_ll_lr_rl_rr_planes();
    accepted_pcm32_and_float32_decode_to_finite_canonical_samples();
    invalid_containers_formats_layouts_and_samples_fail_closed();
    prepared_duration_is_bounded();
}
