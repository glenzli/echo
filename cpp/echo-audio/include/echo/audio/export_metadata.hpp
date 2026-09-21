#pragma once
#include <cstddef>
#include <string_view>
#include <vector>
namespace echo::audio {
/// Container transport only. Rust owns the source-disclosure schema.
void validate_export_comment(std::string_view comment);
/// Optional, padded RIFF LIST/INFO/ICMT chunk; leaves PCM bytes untouched.
[[nodiscard]] std::vector<std::byte> wav_comment_chunk(std::string_view comment);
} // namespace echo::audio
