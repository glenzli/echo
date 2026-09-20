#include "prepared_pcm_reader.hpp"

#include <algorithm>
#include <cstring>
#include <stdexcept>
#include <utility>

namespace echo::audio {
namespace {
std::uint32_t little(const unsigned char* value, unsigned size) {
    std::uint32_t result = 0;
    for (unsigned i = 0; i < size; ++i)
        result |= static_cast<std::uint32_t>(value[i]) << (i * 8);
    return result;
}
} // namespace

std::unique_ptr<PreparedPcmReader> PreparedPcmReader::open(const std::string& path) {
    std::ifstream input(path, std::ios::binary);
    std::array<unsigned char, 44> header{};
    if (!input.read(
            reinterpret_cast<char*>(header.data()),
            static_cast<std::streamsize>(header.size())
        ))
        return nullptr;
    const auto* h = header.data();
    if (std::memcmp(h, "RIFF", 4) || std::memcmp(h + 8, "WAVEfmt ", 8) || little(h + 16, 4) != 16
        || little(h + 20, 2) != 1 || little(h + 22, 2) != 2 || little(h + 24, 4) != 48000
        || std::memcmp(h + 36, "data", 4))
        return nullptr;
    const auto bits = little(h + 34, 2);
    if (bits != 16 && bits != 24)
        return nullptr;
    const unsigned sample_bytes = bits / 8;
    const auto bytes = little(h + 40, 4);
    if (little(h + 28, 4) != 48000 * 2 * sample_bytes || little(h + 32, 2) != 2 * sample_bytes
        || bytes % (2 * sample_bytes)
        || static_cast<std::uint64_t>(little(h + 4, 4)) != 36ULL + bytes)
        throw std::runtime_error("invalid prepared PCM header");
    input.seekg(0, std::ios::end);
    if (!input || input.tellg() < static_cast<std::streamoff>(44ULL + bytes))
        throw std::runtime_error("truncated prepared PCM source");
    return std::unique_ptr<PreparedPcmReader>(
        new PreparedPcmReader(std::move(input), bytes, sample_bytes)
    );
}

PreparedPcmReader::PreparedPcmReader(
    std::ifstream input,
    std::uint32_t bytes,
    unsigned sample_bytes
) : input_(std::move(input)), frames_(bytes / (2 * sample_bytes)), sample_bytes_(sample_bytes) {
    seek(0);
}

void PreparedPcmReader::seek(std::uint64_t frame) {
    cursor_ = std::min(frame, frames_);
    input_.clear();
    input_.seekg(static_cast<std::streamoff>(44 + cursor_ * 2 * sample_bytes_));
    if (!input_)
        throw std::runtime_error("cannot seek prepared PCM source");
}

std::size_t PreparedPcmReader::read(std::span<float> stereo) {
    const auto frames = static_cast<std::size_t>(
        std::min<std::uint64_t>({frames_ - cursor_, stereo.size() / 2, 4096})
    );
    const auto count = frames * 2 * sample_bytes_;
    if (!input_.read(reinterpret_cast<char*>(bytes_.data()), static_cast<std::streamsize>(count)))
        throw std::runtime_error("cannot read prepared PCM source");
    const std::uint32_t sign = 1U << (sample_bytes_ * 8 - 1);
    const float scale = 1.0F / static_cast<float>(sign);
    for (std::size_t i = 0; i < frames * 2; ++i) {
        const auto raw = little(bytes_.data() + i * sample_bytes_, sample_bytes_);
        const auto sample = static_cast<std::int32_t>(raw)
                            - ((raw & sign) ? static_cast<std::int32_t>(sign * 2) : 0);
        stereo[i] = static_cast<float>(sample) * scale;
    }
    cursor_ += frames;
    return frames;
}
} // namespace echo::audio
