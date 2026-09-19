#include "ffmpeg_input.hpp"
#include <array>
#include <fstream>
#include <string_view>

namespace echo::audio {
int open_audio_input(AVFormatContext** context, const std::string& path) {
    std::array<char, 12> header{};
    std::ifstream source(path, std::ios::binary);
    source.read(header.data(), static_cast<std::streamsize>(header.size()));
    const std::string_view container(header.data(), 4), kind(header.data() + 8, 4);
    const bool wave = source.gcount() == static_cast<std::streamsize>(header.size())
                      && (container == "RIFF" || container == "RF64" || container == "RIFX")
                      && kind == "WAVE";
    return avformat_open_input(
        context,
        path.c_str(),
        wave ? av_find_input_format("wav") : nullptr,
        nullptr
    );
}
} // namespace echo::audio
