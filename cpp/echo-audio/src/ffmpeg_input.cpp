#include "ffmpeg_input.hpp"
#include "ffmpeg_timing.hpp"
#include <algorithm>
#include <array>
#include <fstream>
#include <string_view>

namespace echo::audio {
int64_t audio_start_time(const AVFormatContext* context, const AVStream* stream) {
    if (const auto timing = exact_audio_timing(context, stream))
        return timing->first_timestamp;
    return stream->start_time == AV_NOPTS_VALUE ? 0 : stream->start_time;
}
int64_t audio_duration(const AVFormatContext* context, const AVStream* stream, AVRational units) {
    if (const auto timing = exact_audio_timing(context, stream))
        return av_rescale_q(timing->samples, AVRational{1, timing->sample_rate}, units);
    if (stream->duration != AV_NOPTS_VALUE && stream->duration > 0)
        return av_rescale_q(stream->duration, stream->time_base, units);
    const auto* tag = av_dict_get(stream->metadata, "DURATION", nullptr, 0);
    int64_t micros = 0;
    if (tag && av_parse_time(&micros, tag->value, 1) >= 0 && micros > 0) {
        // Matroska's DURATION tag is the track's end timestamp, including an offset.
        const auto start = stream->start_time == AV_NOPTS_VALUE
                               ? 0
                               : std::max<int64_t>(
                                     0,
                                     av_rescale_q(
                                         stream->start_time,
                                         stream->time_base,
                                         AVRational{1, AV_TIME_BASE}
                                     )
                                 );
        return av_rescale_q(
            std::max<int64_t>(0, micros - start),
            AVRational{1, AV_TIME_BASE},
            units
        );
    }
    if (context->duration != AV_NOPTS_VALUE && context->duration > 0)
        return av_rescale_q(context->duration, AVRational{1, AV_TIME_BASE}, units);
    return 0;
}
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
