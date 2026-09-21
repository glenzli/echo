#include "echo/audio/export_metadata.hpp"
#include <cstdint>
#include <cstring>
#include <stdexcept>
namespace echo::audio {
void validate_export_comment(std::string_view comment) {
    if (comment.size() > 1024 || comment.find('\0') != std::string_view::npos) {
        throw std::invalid_argument("export comment is too large or invalid");
    }
}
std::vector<std::byte> wav_comment_chunk(std::string_view comment) {
    validate_export_comment(comment);
    if (comment.empty()) {
        return {};
    }
    const auto payload = static_cast<std::uint32_t>(comment.size() + 1);
    const auto padded = payload + (payload & 1U);
    std::vector<std::byte> bytes(20U + padded);
    auto number = [&](std::size_t offset, std::uint32_t value) {
        for (std::size_t i = 0; i < 4; ++i) {
            bytes[offset + i] = std::byte{static_cast<unsigned char>((value >> (8U * i)) & 0xffU)};
        }
    };
    std::memcpy(bytes.data(), "LIST", 4);
    number(4, 12U + padded);
    std::memcpy(bytes.data() + 8, "INFOICMT", 8);
    number(16, payload);
    std::memcpy(bytes.data() + 20, comment.data(), comment.size());
    return bytes;
}
} // namespace echo::audio
