#include "echo/audio/decode.hpp"
#include "ffmpeg_input.hpp"

#include "echo/audio/ffmpeg_include.hpp"

#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

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

constexpr std::size_t kMaximumMetadataEntries = 128;
constexpr std::size_t kMaximumMetadataValueBytes = 4096;

void append_metadata(
    std::vector<AudioMetadataEntry>& destination,
    const AVDictionary* dictionary,
    std::string_view prefix
) {
    const AVDictionaryEntry* entry = nullptr;
    while (destination.size() < kMaximumMetadataEntries
           && (entry = av_dict_get(dictionary, "", entry, AV_DICT_IGNORE_SUFFIX)) != nullptr) {
        std::string value = entry->value != nullptr ? entry->value : "";
        if (value.size() > kMaximumMetadataValueBytes) {
            value.resize(kMaximumMetadataValueBytes);
        }
        destination.push_back(
            AudioMetadataEntry{
                .key = std::string(prefix) + (entry->key != nullptr ? entry->key : ""),
                .value = std::move(value),
            }
        );
    }
}

int64_t recorded_at_millis(const AVDictionary* container, const AVDictionary* stream) {
    const AVDictionaryEntry* entry = av_dict_get(container, "creation_time", nullptr, 0);
    if (entry == nullptr) {
        entry = av_dict_get(stream, "creation_time", nullptr, 0);
    }
    int64_t timestamp_micros = 0;
    if (entry == nullptr || entry->value == nullptr
        || av_parse_time(&timestamp_micros, entry->value, 0) < 0 || timestamp_micros <= 0) {
        return 0;
    }
    return timestamp_micros / 1000;
}

} // namespace

AudioProbe probe(const std::string& path) {
    FormatContext format;
    int result = open_audio_input(format.slot(), path);
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
        probe_result.recorded_at_millis =
            recorded_at_millis(format.get()->metadata, stream->metadata);
        append_metadata(probe_result.metadata, format.get()->metadata, "");
        append_metadata(probe_result.metadata, stream->metadata, "stream.");
        break;
    }
    return probe_result;
}

} // namespace echo::audio
