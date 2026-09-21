#pragma once
#include "echo/audio/ffmpeg_include.hpp"
#include <string>

namespace echo::audio {
// File signatures take precedence over payload heuristics for RIFF-family WAVE.
// Other containers retain FFmpeg's normal probing and validation.
int open_audio_input(AVFormatContext** context, const std::string& path);
// Matroska/WebM commonly store duration as a track tag or only on the container.
// Return zero for unknown duration, never rescale AV_NOPTS_VALUE into unsigned time.
int64_t audio_start_time(const AVFormatContext* context, const AVStream* stream);
int64_t audio_duration(const AVFormatContext* context, const AVStream* stream, AVRational units);
} // namespace echo::audio
