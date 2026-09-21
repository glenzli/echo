#pragma once
#include "echo/audio/offline_render.hpp"
#include <string>
#include <vector>
namespace echo::audio {
struct AudioStreamInfo {
    int index = -1;
    std::string codec, title, language;
    int sample_rate = 0, channels = 0;
    std::uint64_t duration_millis = 0;
};
std::vector<AudioStreamInfo> audio_streams(const std::string& path);
// Packet-preserving extraction. The complete original container is retained by the caller.
void copy_audio_stream(
    const std::string& path,
    int index,
    RenderByteSink& sink,
    const std::string& source_sha256,
    const std::string& source_filename,
    const OfflineRenderCallbacks& callbacks = {}
);
} // namespace echo::audio
