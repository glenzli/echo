#include "echo/audio/prepared_impulse_response.hpp"

#include "echo/audio/impulse_response_preparer.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <limits>
#include <stdexcept>
#include <string>

namespace echo::audio {
namespace {

constexpr std::uint64_t kMaximumPreparedFrames =
    5U * static_cast<std::uint64_t>(kPreparedImpulseResponseSampleRate);
constexpr std::uint32_t kMinimumSourceSampleRate = 8000;
constexpr std::uint32_t kMaximumSourceSampleRate = 192000;

[[noreturn]] void fail(const std::string& message) {
    throw std::runtime_error(message);
}

std::uint32_t read_u32(const std::uint8_t* bytes) {
    return static_cast<std::uint32_t>(bytes[0]) | (static_cast<std::uint32_t>(bytes[1]) << 8U)
           | (static_cast<std::uint32_t>(bytes[2]) << 16U)
           | (static_cast<std::uint32_t>(bytes[3]) << 24U);
}

std::uint64_t read_u64(const std::uint8_t* bytes) {
    return read_u32(bytes) | (static_cast<std::uint64_t>(read_u32(bytes + 4)) << 32U);
}

void read_exact(std::ifstream& input, void* destination, std::size_t size) {
    input.read(static_cast<char*>(destination), static_cast<std::streamsize>(size));
    if (input.gcount() != static_cast<std::streamsize>(size)) {
        fail("prepared impulse response is truncated");
    }
}

float read_float(std::ifstream& input) {
    std::array<std::uint8_t, 4> bytes{};
    read_exact(input, bytes.data(), bytes.size());
    return std::bit_cast<float>(read_u32(bytes.data()));
}

std::uint64_t checked_data_bytes(std::uint64_t frames, std::uint32_t channels) {
    if (channels == 0U
        || frames > std::numeric_limits<std::uint64_t>::max() / channels / sizeof(float)) {
        fail("prepared impulse response size overflows");
    }
    return frames * channels * sizeof(float);
}

} // namespace

LoadedPreparedImpulseResponse load_prepared_impulse_response(const std::string& path) {
    const std::uint64_t file_size = std::filesystem::file_size(path);
    if (file_size < kPreparedImpulseResponseHeaderBytes) {
        fail("prepared impulse response is smaller than its header");
    }
    std::ifstream input(path, std::ios::binary);
    if (!input) {
        fail("cannot open prepared impulse response " + path);
    }
    std::array<std::uint8_t, kPreparedImpulseResponseHeaderBytes> header{};
    read_exact(input, header.data(), header.size());
    if (!std::equal(
            header.begin(),
            header.begin() + 8,
            reinterpret_cast<const std::uint8_t*>("ECHOIR01")
        )) {
        fail("prepared impulse response has an unknown identity");
    }

    const std::uint32_t header_bytes = read_u32(header.data() + 8);
    const std::uint32_t preparation_version = read_u32(header.data() + 12);
    const std::uint32_t sample_rate = read_u32(header.data() + 16);
    const std::uint32_t channel_count = read_u32(header.data() + 20);
    const std::uint64_t frame_count = read_u64(header.data() + 24);
    const std::uint32_t source_sample_rate = read_u32(header.data() + 32);
    const std::uint32_t source_channel_count = read_u32(header.data() + 36);
    const std::uint64_t source_frame_count = read_u64(header.data() + 40);
    const std::uint32_t avcodec_version = read_u32(header.data() + 48);
    const std::uint32_t swresample_version = read_u32(header.data() + 52);
    const std::uint64_t data_bytes = read_u64(header.data() + 56);

    const bool supported_layout =
        (preparation_version == kPreparedImpulseResponseVersion
         && (channel_count == 1U || channel_count == 2U))
        || (preparation_version == kPreparedTrueStereoImpulseResponseVersion
            && channel_count == 4U);
    if (header_bytes != kPreparedImpulseResponseHeaderBytes || !supported_layout
        || sample_rate != kPreparedImpulseResponseSampleRate
        || source_channel_count != channel_count || frame_count == 0U
        || frame_count > kMaximumPreparedFrames || source_frame_count == 0U
        || source_frame_count > 5U * static_cast<std::uint64_t>(source_sample_rate)
        || source_sample_rate < kMinimumSourceSampleRate
        || source_sample_rate > kMaximumSourceSampleRate || avcodec_version == 0U
        || swresample_version == 0U) {
        fail("prepared impulse response header is unsupported");
    }
    const std::uint64_t expected_data_bytes = checked_data_bytes(frame_count, channel_count);
    if (data_bytes != expected_data_bytes
        || file_size != kPreparedImpulseResponseHeaderBytes + expected_data_bytes) {
        fail("prepared impulse response byte count is inconsistent");
    }

    const bool true_stereo = channel_count == 4U;
    LoadedPreparedImpulseResponse loaded{
        .preparation_version = preparation_version,
        .source_sample_rate = source_sample_rate,
        .source_frame_count = source_frame_count,
        .avcodec_version = avcodec_version,
        .swresample_version = swresample_version,
        .layout = true_stereo           ? PreparedImpulseLayout::TrueStereoLlLrRlRr
                  : channel_count == 2U ? PreparedImpulseLayout::StereoParallel
                                        : PreparedImpulseLayout::Mono,
        .left = std::vector<float>(static_cast<std::size_t>(frame_count)),
        .right = channel_count >= 2U ? std::vector<float>(static_cast<std::size_t>(frame_count))
                                     : std::vector<float>{},
        .left_to_right = true_stereo ? std::vector<float>(static_cast<std::size_t>(frame_count))
                                     : std::vector<float>{},
        .right_to_left = true_stereo ? std::vector<float>(static_cast<std::size_t>(frame_count))
                                     : std::vector<float>{},
    };
    bool any_nonzero = false;
    const auto read_channel = [&](std::vector<float>& channel) {
        for (float& sample : channel) {
            sample = read_float(input);
            if (!std::isfinite(sample)) {
                fail("prepared impulse response contains a non-finite sample");
            }
            any_nonzero = any_nonzero || sample != 0.0F;
        }
    };
    read_channel(loaded.left);
    if (true_stereo) {
        read_channel(loaded.left_to_right);
        read_channel(loaded.right_to_left);
    }
    read_channel(loaded.right);
    if (!any_nonzero) {
        fail("prepared impulse response is digital silence");
    }
    return loaded;
}

} // namespace echo::audio
