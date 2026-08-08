#include "echo/audio/waveform.hpp"

#include "echo/audio/ffmpeg_include.hpp"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>

namespace echo::audio {
namespace {

constexpr int kCanonicalSampleRate = 48000;
constexpr int kBaseBucketSamples = 480; // 10 ms at the canonical rate

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

struct DecoderContextDeleter {
    void operator()(AVCodecContext* context) const {
        avcodec_free_context(&context);
    }
};

struct PacketDeleter {
    void operator()(AVPacket* packet) const {
        av_packet_free(&packet);
    }
};

struct FrameDeleter {
    void operator()(AVFrame* frame) const {
        av_frame_free(&frame);
    }
};

struct SwrDeleter {
    void operator()(SwrContext* context) const {
        swr_free(&context);
    }
};

/// Streaming decoder over one audio stream, resampled to the canonical mono
/// float mixdown at 48 kHz. Owns its codec context, packets, frames, and
/// resampler; samples are processed immediately and never retained.
class CanonicalMonoDecoder {
  public:
    explicit CanonicalMonoDecoder(const AVStream* stream) {
        const AVCodecParameters* codecpar = stream->codecpar;
        const AVCodec* decoder = avcodec_find_decoder(codecpar->codec_id);
        if (decoder == nullptr) {
            fail("no decoder for codec " + std::string(avcodec_get_name(codecpar->codec_id)));
        }
        codec_.reset(avcodec_alloc_context3(decoder));
        if (codec_ == nullptr) {
            fail("cannot allocate decoder context");
        }
        int result = avcodec_parameters_to_context(codec_.get(), codecpar);
        if (result < 0) {
            fail("cannot initialize decoder context: " + av_error_text(result));
        }
        result = avcodec_open2(codec_.get(), decoder, nullptr);
        if (result < 0) {
            fail("cannot open decoder: " + av_error_text(result));
        }

        AVChannelLayout output_layout;
        av_channel_layout_default(&output_layout, 1);
        SwrContext* raw_swr = swr_alloc();
        if (raw_swr == nullptr) {
            fail("cannot allocate resampler");
        }
        swr_.reset(raw_swr);
        result = swr_alloc_set_opts2(
            &raw_swr,
            &output_layout,
            AV_SAMPLE_FMT_FLTP,
            kCanonicalSampleRate,
            &codec_->ch_layout,
            codec_->sample_fmt,
            codec_->sample_rate,
            0,
            nullptr
        );
        if (result < 0) {
            fail("cannot configure resampler: " + av_error_text(result));
        }
        result = swr_init(raw_swr);
        if (result < 0) {
            fail("cannot initialize resampler: " + av_error_text(result));
        }

        frame_.reset(av_frame_alloc());
        if (frame_ == nullptr) {
            fail("cannot allocate decode buffers");
        }
    }

    /// Decodes one packet into samples via `consume`.
    template <typename Consumer> void drain_packet(AVPacket* packet, Consumer&& consume) {
        if (avcodec_send_packet(codec_.get(), packet) < 0) {
            return;
        }
        receive_frames(consume);
    }

    /// Flushes remaining buffered frames after end-of-stream.
    template <typename Consumer> void flush(Consumer&& consume) {
        avcodec_send_packet(codec_.get(), nullptr);
        receive_frames(consume);
    }

  private:
    template <typename Consumer> void receive_frames(Consumer&& consume) {
        while (avcodec_receive_frame(codec_.get(), frame_.get()) == 0) {
            // Size by the exact resampled output count: upsampling needs more
            // output samples than input frames (see playback.cpp).
            const int output_samples = swr_get_out_samples(swr_.get(), frame_->nb_samples);
            uint8_t* output_data[1] = {nullptr};
            int output_linesize = 0;
            const int allocation_result = av_samples_alloc(
                output_data,
                &output_linesize,
                1,
                output_samples,
                AV_SAMPLE_FMT_FLTP,
                0
            );
            if (allocation_result < 0) {
                fail("cannot allocate resampler output: " + av_error_text(allocation_result));
            }
            const int sample_count = swr_convert(
                swr_.get(),
                output_data,
                output_samples,
                const_cast<const uint8_t**>(frame_->extended_data),
                frame_->nb_samples
            );
            if (sample_count > 0) {
                const float* samples = reinterpret_cast<const float*>(output_data[0]);
                for (int index = 0; index < sample_count; ++index) {
                    consume(samples[index]);
                }
            }
            av_freep(&output_data[0]);
        }
        av_frame_unref(frame_.get());
    }

    std::unique_ptr<AVCodecContext, DecoderContextDeleter> codec_;
    std::unique_ptr<SwrContext, SwrDeleter> swr_;
    std::unique_ptr<AVFrame, FrameDeleter> frame_;
};

void append_combined_level(
    Waveform& waveform,
    const std::vector<float>& mins,
    const std::vector<float>& maxs,
    uint32_t samples_per_bucket
) {
    WaveformLevel level;
    level.mins = mins;
    level.maxs = maxs;
    level.samples_per_bucket = samples_per_bucket;
    waveform.levels.push_back(std::move(level));
}

std::pair<std::vector<float>, std::vector<float>>
halve_level(const std::vector<float>& mins, const std::vector<float>& maxs) {
    std::vector<float> next_mins;
    std::vector<float> next_maxs;
    next_mins.reserve((mins.size() + 1) / 2);
    next_maxs.reserve((maxs.size() + 1) / 2);
    for (std::size_t index = 0; index + 1 < mins.size(); index += 2) {
        next_mins.push_back(std::min(mins[index], mins[index + 1]));
        next_maxs.push_back(std::max(maxs[index], maxs[index + 1]));
    }
    return {std::move(next_mins), std::move(next_maxs)};
}

} // namespace

Waveform build_waveform(const std::string& path, uint32_t max_levels) {
    if (max_levels == 0) {
        fail("max_levels must be positive");
    }
    FormatContext format;
    int result = avformat_open_input(format.slot(), path.c_str(), nullptr, nullptr);
    if (result < 0) {
        fail("cannot open " + path + ": " + av_error_text(result));
    }
    result = avformat_find_stream_info(format.get(), nullptr);
    if (result < 0) {
        fail("cannot read stream info for " + path + ": " + av_error_text(result));
    }

    const AVStream* audio_stream = nullptr;
    for (unsigned int index = 0; index < format.get()->nb_streams; ++index) {
        const AVStream* candidate = format.get()->streams[index];
        if (candidate->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            audio_stream = candidate;
            break;
        }
    }
    if (audio_stream == nullptr) {
        fail("no audio stream in " + path);
    }

    CanonicalMonoDecoder decoder(audio_stream);

    std::vector<float> base_mins;
    std::vector<float> base_maxs;
    base_mins.reserve(65536);
    base_maxs.reserve(65536);
    float bucket_min = std::numeric_limits<float>::infinity();
    float bucket_max = -std::numeric_limits<float>::infinity();
    int bucket_samples = 0;

    const auto flush_bucket = [&] {
        if (bucket_samples == 0) {
            return;
        }
        base_mins.push_back(bucket_min);
        base_maxs.push_back(bucket_max);
        bucket_min = std::numeric_limits<float>::infinity();
        bucket_max = -std::numeric_limits<float>::infinity();
        bucket_samples = 0;
    };

    std::unique_ptr<AVPacket, PacketDeleter> packet(av_packet_alloc());
    while (av_read_frame(format.get(), packet.get()) >= 0) {
        if (packet->stream_index == audio_stream->index) {
            decoder.drain_packet(packet.get(), [&](float sample) {
                bucket_min = std::min(bucket_min, sample);
                bucket_max = std::max(bucket_max, sample);
                ++bucket_samples;
                if (bucket_samples >= kBaseBucketSamples) {
                    flush_bucket();
                }
            });
        }
        av_packet_unref(packet.get());
    }
    decoder.flush([&](float sample) {
        bucket_min = std::min(bucket_min, sample);
        bucket_max = std::max(bucket_max, sample);
        ++bucket_samples;
        if (bucket_samples >= kBaseBucketSamples) {
            flush_bucket();
        }
    });
    flush_bucket();

    Waveform waveform;
    waveform.canonical_sample_rate = kCanonicalSampleRate;
    append_combined_level(waveform, base_mins, base_maxs, kBaseBucketSamples);

    std::vector<float> current_mins = std::move(base_mins);
    std::vector<float> current_maxs = std::move(base_maxs);
    uint32_t samples_per_bucket = kBaseBucketSamples;
    while (current_mins.size() > 1 && waveform.levels.size() < max_levels) {
        auto [next_mins, next_maxs] = halve_level(current_mins, current_maxs);
        samples_per_bucket *= 2;
        append_combined_level(waveform, next_mins, next_maxs, samples_per_bucket);
        current_mins = std::move(next_mins);
        current_maxs = std::move(next_maxs);
    }
    return waveform;
}

} // namespace echo::audio
