#include "echo/audio/spectrogram.hpp"
#include "ffmpeg_input.hpp"

#include "convolution/signalsmith_audiofft_adapter.hpp"
#include "echo/audio/ffmpeg_include.hpp"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <memory>
#include <numbers>
#include <stdexcept>
#include <string>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kCanonicalSampleRate = 48'000;
constexpr std::size_t kWindowFrames = 2'048;
constexpr std::size_t kHopFrames = 512;
constexpr std::size_t kSpectrumBins = kWindowFrames / 2 + 1;
constexpr float kMagnitudeFloorDecibels = -96.0F;

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

class CanonicalMonoDecoder {
  public:
    explicit CanonicalMonoDecoder(const AVStream* stream) {
        const AVCodecParameters* codec_parameters = stream->codecpar;
        const AVCodec* decoder = avcodec_find_decoder(codec_parameters->codec_id);
        if (decoder == nullptr) {
            fail("no decoder for " + std::string(avcodec_get_name(codec_parameters->codec_id)));
        }
        codec_.reset(avcodec_alloc_context3(decoder));
        if (codec_ == nullptr) {
            fail("cannot allocate decoder context");
        }
        int result = avcodec_parameters_to_context(codec_.get(), codec_parameters);
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

    template <typename Consumer> void drain_packet(AVPacket* packet, Consumer&& consume) {
        if (avcodec_send_packet(codec_.get(), packet) >= 0) {
            receive_frames(consume);
        }
    }
    template <typename Consumer> void flush(Consumer&& consume) {
        avcodec_send_packet(codec_.get(), nullptr);
        receive_frames(consume);
    }

  private:
    template <typename Consumer> void receive_frames(Consumer&& consume) {
        while (avcodec_receive_frame(codec_.get(), frame_.get()) == 0) {
            const int output_samples = swr_get_out_samples(swr_.get(), frame_->nb_samples);
            uint8_t* output_data[1] = {nullptr};
            int output_linesize = 0;
            const int allocation = av_samples_alloc(
                output_data,
                &output_linesize,
                1,
                output_samples,
                AV_SAMPLE_FMT_FLTP,
                0
            );
            if (allocation < 0) {
                fail("cannot allocate resampler output: " + av_error_text(allocation));
            }
            const int sample_count = swr_convert(
                swr_.get(),
                output_data,
                output_samples,
                const_cast<const uint8_t**>(frame_->extended_data),
                frame_->nb_samples
            );
            if (sample_count > 0) {
                const auto* samples = reinterpret_cast<const float*>(output_data[0]);
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

void reduce_columns(std::vector<std::uint8_t>& columns, std::size_t bins) {
    std::vector<std::uint8_t> reduced;
    reduced.reserve(((columns.size() / bins) + 1) / 2 * bins);
    for (std::size_t offset = 0; offset < columns.size(); offset += 2 * bins) {
        const bool has_second = offset + 2 * bins <= columns.size();
        for (std::size_t bin = 0; bin < bins; ++bin) {
            const std::uint8_t first = columns[offset + bin];
            const std::uint8_t second = has_second ? columns[offset + bins + bin] : first;
            reduced.push_back(std::max(first, second));
        }
    }
    columns = std::move(reduced);
}

} // namespace

SpectrogramOverview build_spectrogram_overview(
    const std::string& path,
    std::uint32_t max_time_columns,
    std::uint32_t frequency_bins
) {
    if (max_time_columns == 0 || frequency_bins == 0 || frequency_bins > kSpectrumBins) {
        throw std::invalid_argument("spectrogram dimensions are outside the supported range");
    }
    FormatContext format;
    int result = open_audio_input(format.slot(), path);
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
    audiofft::AudioFFT fft;
    fft.init(kWindowFrames);
    std::vector<float> window(kWindowFrames);
    std::vector<float> real(kSpectrumBins);
    std::vector<float> imaginary(kSpectrumBins);
    std::vector<float> history(kWindowFrames, 0.0F);
    std::vector<std::uint8_t> columns;
    columns.reserve(static_cast<std::size_t>(max_time_columns) * frequency_bins * 2);
    std::size_t cursor = 0;
    std::uint64_t received = 0;

    const auto emit_column = [&] {
        for (std::size_t frame = 0; frame < kWindowFrames; ++frame) {
            const std::size_t source = (cursor + frame) % kWindowFrames;
            const float phase = 2.0F * std::numbers::pi_v<float>
                                * static_cast<float>(frame) / static_cast<float>(kWindowFrames);
            window[frame] = history[source] * (0.5F - 0.5F * std::cos(phase));
        }
        fft.fft(window.data(), real.data(), imaginary.data());
        for (std::size_t row = 0; row < frequency_bins; ++row) {
            const std::size_t start = row * kSpectrumBins / frequency_bins;
            const std::size_t end = (row + 1) * kSpectrumBins / frequency_bins;
            float maximum = kMagnitudeFloorDecibels;
            for (std::size_t bin = start; bin < std::max(start + 1, end); ++bin) {
                const float magnitude = std::hypot(real[bin], imaginary[bin]);
                maximum = std::max(maximum, 20.0F * std::log10(std::max(magnitude, 1.0E-8F)));
            }
            const float normalized = std::clamp(
                (maximum - kMagnitudeFloorDecibels) / -kMagnitudeFloorDecibels,
                0.0F,
                1.0F
            );
            columns.push_back(static_cast<std::uint8_t>(std::lround(normalized * 255.0F)));
        }
        if (columns.size() / frequency_bins > static_cast<std::size_t>(max_time_columns) * 2) {
            reduce_columns(columns, frequency_bins);
        }
    };

    const auto consume = [&](float sample) {
        history[cursor] = std::isfinite(sample) ? sample : 0.0F;
        cursor = (cursor + 1) % kWindowFrames;
        ++received;
        if (received >= kWindowFrames && (received - kWindowFrames) % kHopFrames == 0) {
            emit_column();
        }
    };
    std::unique_ptr<AVPacket, PacketDeleter> packet(av_packet_alloc());
    while (av_read_frame(format.get(), packet.get()) >= 0) {
        if (packet->stream_index == audio_stream->index) {
            decoder.drain_packet(packet.get(), consume);
        }
        av_packet_unref(packet.get());
    }
    decoder.flush(consume);
    while (columns.size() / frequency_bins > max_time_columns) {
        reduce_columns(columns, frequency_bins);
    }
    return SpectrogramOverview{
        .canonical_sample_rate = kCanonicalSampleRate,
        .window_frames = static_cast<std::uint32_t>(kWindowFrames),
        .hop_frames = static_cast<std::uint32_t>(kHopFrames),
        .time_columns = static_cast<std::uint32_t>(columns.size() / frequency_bins),
        .frequency_bins = frequency_bins,
        .magnitudes = std::move(columns),
    };
}

} // namespace echo::audio
