#pragma once
#include "echo/audio/ffmpeg_include.hpp"
#include <cstdint>
#include <optional>
namespace echo::audio {
struct ExactAudioTiming {
    std::int64_t samples;
    int sample_rate;
    std::int64_t first_timestamp;
};
// ADTS bitrate estimates, ASF delay and Opus pre-skip are insufficient for exact source edits.
// Decode these local containers once with bounded buffers; cache only scalar timing.
std::optional<ExactAudioTiming> exact_audio_timing(const AVFormatContext*, const AVStream*);
} // namespace echo::audio
