#pragma once
#include "echo/audio/ffmpeg_include.hpp"
#include <string>

namespace echo::audio {
// File signatures take precedence over payload heuristics for RIFF-family WAVE.
// Other containers retain FFmpeg's normal probing and validation.
int open_audio_input(AVFormatContext** context, const std::string& path);
} // namespace echo::audio
