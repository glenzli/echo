#include "echo/audio/decode.hpp"

#include "echo/audio/ffmpeg_include.hpp"

#include <stdexcept>
#include <string>

namespace echo::audio {
namespace {

std::string av_error_text(int code) {
    char buffer[AV_ERROR_MAX_STRING_SIZE] = {0};
    av_strerror(code, buffer, sizeof(buffer));
    return buffer;
}

[[noreturn]] void fail(const std::string& message) {
    throw std::runtime_error(message);
}

class FormatContext {
  public:
    ~FormatContext() {
        if (ptr_ != nullptr) {
            avformat_close_input(&ptr_);
        }
    }

    AVFormatContext** slot() {
        return &ptr_;
    }
    AVFormatContext* get() const {
        return ptr_;
    }

  private:
    AVFormatContext* ptr_ = nullptr;
};

} // namespace

AudioProbe probe(const std::string& path) {
    FormatContext format;
    int result = avformat_open_input(format.slot(), path.c_str(), nullptr, nullptr);
    if (result < 0) {
        fail("cannot open " + path + ": " + av_error_text(result));
    }
    result = avformat_find_stream_info(format.get(), nullptr);
    if (result < 0) {
        fail("cannot read stream info for " + path + ": " + av_error_text(result));
    }

    AudioProbe probe_result;
    for (unsigned int index = 0; index < format.get()->nb_streams; ++index) {
        const AVStream* stream = format.get()->streams[index];
        if (stream->codecpar->codec_type != AVMEDIA_TYPE_AUDIO) {
            continue;
        }
        probe_result.has_audio = true;
        const AVCodecParameters* codecpar = stream->codecpar;
        probe_result.codec_name = codecpar->codec_id != AV_CODEC_ID_NONE
                                      ? avcodec_get_name(codecpar->codec_id)
                                      : "unknown";
        probe_result.container_format =
            format.get()->iformat != nullptr ? format.get()->iformat->name : "unknown";
        probe_result.sample_rate = static_cast<uint32_t>(codecpar->sample_rate);
        probe_result.channel_count = static_cast<uint32_t>(codecpar->ch_layout.nb_channels);
        if (stream->duration != AV_NOPTS_VALUE) {
            const int64_t duration =
                av_rescale_q(stream->duration, stream->time_base, AVRational{1, 1000});
            if (duration > 0) {
                probe_result.duration_millis = static_cast<uint64_t>(duration);
            }
        }
        break;
    }
    return probe_result;
}

} // namespace echo::audio
