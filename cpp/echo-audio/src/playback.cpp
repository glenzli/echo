#include "echo/audio/playback.hpp"

#include "echo/audio/algorithmic_reverb.hpp"
#include "echo/audio/dynamics_processor.hpp"
#include "echo/audio/ffmpeg_include.hpp"
#include "echo/audio/loudness_meter.hpp"
#include "echo/audio/low_cut_filter.hpp"
#include "echo/audio/output_guard.hpp"
#include "echo/audio/output_limiter.hpp"
#include "echo/audio/parametric_equalizer.hpp"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <limits>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace echo::audio {
namespace {

constexpr int kCanonicalSampleRate = 48000;
constexpr std::uint64_t kRingCapacityFrames = 4096; // 85 ms at 48 kHz
constexpr std::uint64_t kSeekWaitTimeoutMillis = 50;
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

/// Fixed-capacity single-producer/single-consumer frame ring.
///
/// Indices are monotonically increasing (wrapping the counter space is
/// impossible at 48 kHz), so "empty" means `write == read` and "full" means
/// `write - read == capacity`; the array offset is `index % capacity`.
/// Only the producer writes `write_index`; only the consumer writes
/// `read_index`.
class FrameRing {
  public:
    FrameRing(std::size_t capacity_frames, std::size_t channels) :
        data_(capacity_frames * channels, 0.0F), capacity_frames_(capacity_frames),
        channels_(channels) {}

    std::size_t write(const float* frames, std::size_t count) {
        std::size_t written = 0;
        while (written < count) {
            const std::size_t write_index = write_index_.load(std::memory_order_relaxed);
            const std::size_t read_index = read_index_.load(std::memory_order_acquire);
            const std::size_t available = capacity_frames_ - (write_index - read_index);
            if (available == 0) {
                break;
            }
            const std::size_t chunk = std::min(count - written, available);
            const std::size_t first = write_index % capacity_frames_;
            const std::size_t contiguous = std::min(chunk, capacity_frames_ - first);
            std::memcpy(
                data_.data() + first * channels_,
                frames + written * channels_,
                contiguous * channels_ * sizeof(float)
            );
            if (chunk > contiguous) {
                std::memcpy(
                    data_.data(),
                    frames + (written + contiguous) * channels_,
                    (chunk - contiguous) * channels_ * sizeof(float)
                );
            }
            write_index_.store(write_index + chunk, std::memory_order_release);
            written += chunk;
        }
        return written;
    }

    std::size_t read(float* output, std::size_t count) {
        std::size_t read_frames = 0;
        while (read_frames < count) {
            const std::size_t read_index = read_index_.load(std::memory_order_relaxed);
            const std::size_t write_index = write_index_.load(std::memory_order_acquire);
            const std::size_t filled = write_index - read_index;
            if (filled == 0) {
                break;
            }
            const std::size_t chunk = std::min(count - read_frames, filled);
            const std::size_t first = read_index % capacity_frames_;
            const std::size_t contiguous = std::min(chunk, capacity_frames_ - first);
            std::memcpy(
                output + read_frames * channels_,
                data_.data() + first * channels_,
                contiguous * channels_ * sizeof(float)
            );
            if (chunk > contiguous) {
                std::memcpy(
                    output + (read_frames + contiguous) * channels_,
                    data_.data(),
                    (chunk - contiguous) * channels_ * sizeof(float)
                );
            }
            read_index_.store(read_index + chunk, std::memory_order_release);
            read_frames += chunk;
        }
        return read_frames;
    }

    /// Discards all buffered frames (seek path; an in-flight consumer copy
    /// finishes with stale-but-bounded data).
    void reset() {
        read_index_.store(write_index_.load(std::memory_order_acquire), std::memory_order_release);
    }

    std::size_t available() const {
        const std::size_t read_index = read_index_.load(std::memory_order_acquire);
        const std::size_t write_index = write_index_.load(std::memory_order_acquire);
        return write_index - read_index;
    }

    std::size_t writable() const {
        return capacity_frames_ - available();
    }

  private:
    std::vector<float> data_;
    std::size_t capacity_frames_;
    std::size_t channels_;
    std::atomic<std::size_t> read_index_{0};
    std::atomic<std::size_t> write_index_{0};
};

} // namespace

class PlaybackSession::Impl {
  public:
    Impl(const std::string& path, PlaybackAdjustment adjustment) : path_(path) {
        AVFormatContext** format_slot = format_.slot();
        int result = avformat_open_input(format_slot, path.c_str(), nullptr, nullptr);
        if (result < 0) {
            fail("cannot open " + path + ": " + av_error_text(result));
        }
        result = avformat_find_stream_info(format_.get(), nullptr);
        if (result < 0) {
            fail("cannot read stream info for " + path + ": " + av_error_text(result));
        }
        const AVStream* stream = find_audio_stream();
        if (stream == nullptr) {
            fail("no audio stream in " + path);
        }
        stream_index_ = stream->index;
        const AVCodecParameters* codecpar = stream->codecpar;
        duration_millis_ = static_cast<std::uint64_t>(
            av_rescale_q(stream->duration, stream->time_base, AVRational{1, 1000})
        );
        adjustment_ = std::make_unique<PreparedAdjustment>(
            adjustment,
            duration_millis_,
            kCanonicalSampleRate
        );

        const AVCodec* decoder = avcodec_find_decoder(codecpar->codec_id);
        if (decoder == nullptr) {
            fail("no decoder for codec " + std::string(avcodec_get_name(codecpar->codec_id)));
        }
        codec_.reset(avcodec_alloc_context3(decoder));
        if (codec_ == nullptr) {
            fail("cannot allocate decoder context");
        }
        result = avcodec_parameters_to_context(codec_.get(), codecpar);
        if (result < 0) {
            fail("cannot initialize decoder context: " + av_error_text(result));
        }
        result = avcodec_open2(codec_.get(), decoder, nullptr);
        if (result < 0) {
            fail("cannot open decoder: " + av_error_text(result));
        }

        // Playback always drives a stereo device sink: mono sources are
        // upmixed by the resampler, stereo sources pass through. Keeping the
        // device format stable lets the Qt controller reuse one sink across
        // plays (disposing CoreAudio units on replay crashes Qt 6.11).
        constexpr std::uint32_t kPlaybackChannels = 2;
        AVChannelLayout output_layout;
        av_channel_layout_default(&output_layout, kPlaybackChannels);
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
        channel_count_ = kPlaybackChannels;
        low_cut_filter_ = std::make_unique<LowCutFilter>(
            adjustment_->low_cut_hertz(),
            kCanonicalSampleRate,
            channel_count_
        );
        equalizer_ = std::make_unique<ParametricEqualizer>(
            adjustment_->equalizer(),
            kCanonicalSampleRate,
            channel_count_
        );
        dynamics_processor_ =
            std::make_unique<DynamicsProcessor>(adjustment_->compressor(), kCanonicalSampleRate);
        reverb_ = std::make_unique<AlgorithmicReverb>(
            adjustment_->reverb(),
            kCanonicalSampleRate,
            channel_count_
        );
        output_limiter_ =
            std::make_unique<OutputLimiter>(adjustment_->limiter(), kCanonicalSampleRate);
        output_guard_ = std::make_unique<OutputGuard>(kCanonicalSampleRate);
        loudness_meter_ = std::make_unique<LoudnessMeter>(kCanonicalSampleRate, channel_count_);
        pending_equalizer_ = adjustment_->equalizer();

        ring_ = std::make_unique<FrameRing>(kRingCapacityFrames, channel_count_);
        packet_.reset(av_packet_alloc());
        frame_.reset(av_frame_alloc());
        if (packet_ == nullptr || frame_ == nullptr) {
            fail("cannot allocate decode buffers");
        }
        perform_seek(adjustment_->trim_start_millis());
    }

    ~Impl() {
        stop();
        if (thread_.joinable()) {
            thread_.join();
        }
    }

    std::size_t read(float* output, std::size_t max_frames) {
        if (stopped_.load(std::memory_order_acquire)) {
            return 0;
        }
        const std::size_t frames = ring_->read(output, max_frames);
        consumed_frames_.fetch_add(frames, std::memory_order_relaxed);
        return frames;
    }

    void pause() {
        std::lock_guard<std::mutex> lock(control_mutex_);
        paused_ = true;
        control_cv_.notify_one();
    }

    void resume() {
        std::lock_guard<std::mutex> lock(control_mutex_);
        paused_ = false;
        control_cv_.notify_one();
    }

    void stop() {
        {
            std::lock_guard<std::mutex> lock(control_mutex_);
            stopped_ = true;
            control_cv_.notify_one();
        }
        if (thread_.joinable()) {
            thread_.join();
        }
    }

    void seek(std::uint64_t millis) {
        const std::uint64_t clamped = adjustment_->clamp_seek_millis(millis);
        {
            std::lock_guard<std::mutex> lock(control_mutex_);
            pending_seek_millis_ = clamped;
            seek_requested_ = true;
            control_cv_.notify_one();
        }
        const auto deadline =
            std::chrono::steady_clock::now() + std::chrono::milliseconds(kSeekWaitTimeoutMillis);
        while (!seek_completed_.load(std::memory_order_acquire)
               && std::chrono::steady_clock::now() < deadline) {
            std::this_thread::yield();
        }
    }

    void update_equalizer(ParametricEqualizerAdjustment adjustment) {
        [[maybe_unused]] const ParametricEqualizer validation(
            adjustment,
            kCanonicalSampleRate,
            channel_count_
        );
        {
            std::lock_guard<std::mutex> lock(dynamics_mutex_);
            pending_equalizer_ = adjustment;
            equalizer_update_pending_ = true;
        }
        control_cv_.notify_one();
    }

    void update_compressor(CompressorAdjustment adjustment) {
        // Validate on the caller thread without disturbing the live detector.
        [[maybe_unused]] const DynamicsProcessor validation(adjustment, kCanonicalSampleRate);
        {
            std::lock_guard<std::mutex> lock(dynamics_mutex_);
            pending_compressor_ = adjustment;
            compressor_update_pending_ = true;
        }
        control_cv_.notify_one();
    }

    void update_limiter(LimiterAdjustment adjustment) {
        [[maybe_unused]] const OutputLimiter validation(adjustment, kCanonicalSampleRate);
        {
            std::lock_guard<std::mutex> lock(dynamics_mutex_);
            pending_limiter_ = adjustment;
            limiter_update_pending_ = true;
        }
        control_cv_.notify_one();
    }

    void update_reverb(ReverbAdjustment adjustment) {
        [[maybe_unused]] const AlgorithmicReverb validation(
            adjustment,
            kCanonicalSampleRate,
            channel_count_
        );
        {
            std::lock_guard<std::mutex> lock(dynamics_mutex_);
            pending_reverb_ = adjustment;
            reverb_update_pending_ = true;
        }
        control_cv_.notify_one();
    }

    bool is_paused() const {
        return paused_.load(std::memory_order_acquire);
    }

    bool is_stopped() const {
        return stopped_.load(std::memory_order_acquire);
    }

    bool is_ended() const {
        return ended_.load(std::memory_order_acquire);
    }

    std::uint64_t position_millis() const {
        const std::uint64_t frames = consumed_frames_.load(std::memory_order_relaxed);
        return frames * 1000 / kCanonicalSampleRate;
    }

    std::size_t buffered_frames() const {
        return ring_->available();
    }

    std::uint64_t duration_millis() const {
        return duration_millis_;
    }
    std::uint32_t sample_rate() const {
        return kCanonicalSampleRate;
    }
    std::uint32_t channel_count() const {
        return channel_count_;
    }

    PlaybackMeterSnapshot meter_snapshot() const {
        return {
            .momentary_lufs = momentary_lufs_.load(std::memory_order_acquire),
            .output_peak_dbfs = output_peak_dbfs_.load(std::memory_order_acquire),
            .gain_reduction_decibels = gain_reduction_decibels_.load(std::memory_order_acquire),
            .limiter_reduction_decibels =
                limiter_reduction_decibels_.load(std::memory_order_acquire),
        };
    }

    /// Starts the producer thread. Called from the public constructor.
    void start() {
        thread_ = std::thread(&Impl::producer_loop, this);
    }

  private:
    const AVStream* find_audio_stream() const {
        for (unsigned int index = 0; index < format_.get()->nb_streams; ++index) {
            const AVStream* candidate = format_.get()->streams[index];
            if (candidate->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
                return candidate;
            }
        }
        return nullptr;
    }

    void perform_seek(std::uint64_t millis) {
        const std::int64_t timestamp = av_rescale_q(
            static_cast<std::int64_t>(millis),
            AVRational{1, 1000},
            format_.get()->streams[stream_index_]->time_base
        );
        const int result =
            av_seek_frame(format_.get(), stream_index_, timestamp, AVSEEK_FLAG_BACKWARD);
        if (result >= 0) {
            avcodec_flush_buffers(codec_.get());
            swr_close(swr_.get());
            if (swr_init(swr_.get()) < 0) {
                fail("cannot reset resampler after seek");
            }
            consumed_frames_.store(millis * kCanonicalSampleRate / 1000, std::memory_order_relaxed);
            decoded_frame_cursor_ = millis * kCanonicalSampleRate / 1000;
            low_cut_filter_->reset();
            equalizer_->reset();
            dynamics_processor_->reset();
            reverb_->reset();
            output_limiter_->reset();
            output_guard_->reset();
            loudness_meter_->reset();
            momentary_lufs_.store(-70.0F, std::memory_order_release);
            output_peak_dbfs_.store(-70.0F, std::memory_order_release);
            gain_reduction_decibels_.store(0.0F, std::memory_order_release);
            limiter_reduction_decibels_.store(0.0F, std::memory_order_release);
        }
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(50);
        while (ring_->available() > 0 && std::chrono::steady_clock::now() < deadline) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        ring_->reset();
    }

    bool decode_and_feed(float* scratch, std::size_t scratch_frames) {
        while (avcodec_receive_frame(codec_.get(), frame_.get()) == 0) {
            // Upsampling (e.g. 24 kHz TTS -> 48 kHz) needs MORE output
            // samples than input frames; sizing by the input count overflows
            // the buffer and corrupts the heap. swr_get_out_samples returns
            // the exact output count including internal resampler delay.
            const int output_samples = swr_get_out_samples(swr_.get(), frame_->nb_samples);
            uint8_t* output_data[2] = {nullptr};
            int output_linesize = 0;
            const int allocation_result = av_samples_alloc(
                output_data,
                &output_linesize,
                static_cast<int>(channel_count_),
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
                const float* const* planes = reinterpret_cast<const float* const*>(output_data);
                const AVStream* stream = format_.get()->streams[stream_index_];
                std::uint64_t frame_start = decoded_frame_cursor_;
                if (frame_->best_effort_timestamp != AV_NOPTS_VALUE) {
                    const std::int64_t rescaled = av_rescale_q(
                        frame_->best_effort_timestamp,
                        stream->time_base,
                        AVRational{1, kCanonicalSampleRate}
                    );
                    if (rescaled >= 0) {
                        frame_start = static_cast<std::uint64_t>(rescaled);
                    }
                }
                const std::uint64_t frame_end =
                    frame_start + static_cast<std::uint64_t>(sample_count);
                decoded_frame_cursor_ = frame_end;
                const std::uint64_t selected_start =
                    std::max(frame_start, adjustment_->start_frame());
                const std::uint64_t selected_end = std::min(frame_end, adjustment_->end_frame());
                const std::size_t input_offset =
                    selected_start < selected_end
                        ? static_cast<std::size_t>(selected_start - frame_start)
                        : 0;
                const std::size_t selected_count =
                    selected_start < selected_end
                        ? static_cast<std::size_t>(selected_end - selected_start)
                        : 0;
                std::size_t written = 0;
                while (written < selected_count) {
                    const std::size_t writable = ring_->writable();
                    if (writable == 0) {
                        std::unique_lock<std::mutex> lock(control_mutex_);
                        control_cv_.wait_for(lock, std::chrono::milliseconds(1));
                        if (stopped_) {
                            av_freep(&output_data[0]);
                            av_frame_unref(frame_.get());
                            return false;
                        }
                        if (seek_requested_) {
                            av_freep(&output_data[0]);
                            av_frame_unref(frame_.get());
                            return false;
                        }
                        continue;
                    }
                    const std::size_t chunk =
                        std::min({selected_count - written, scratch_frames, writable});
                    {
                        std::lock_guard<std::mutex> lock(dynamics_mutex_);
                        if (equalizer_update_pending_) {
                            equalizer_->transition_to(pending_equalizer_);
                            equalizer_update_pending_ = false;
                        }
                        if (compressor_update_pending_) {
                            dynamics_processor_->update(pending_compressor_);
                            compressor_update_pending_ = false;
                        }
                        if (limiter_update_pending_) {
                            output_limiter_->update(pending_limiter_);
                            limiter_update_pending_ = false;
                        }
                        if (reverb_update_pending_) {
                            reverb_->update(pending_reverb_);
                            reverb_update_pending_ = false;
                        }
                    }
                    for (std::size_t index = 0; index < chunk; ++index) {
                        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                            const float filtered = low_cut_filter_->process_sample(
                                planes[channel][input_offset + written + index],
                                channel
                            );
                            const float equalized = equalizer_->process_sample(filtered, channel);
                            scratch[index * channel_count_ + channel] =
                                equalized * adjustment_->gain_amplitude();
                        }
                    }
                    dynamics_processor_->process_interleaved(scratch, chunk, channel_count_);
                    reverb_->process_interleaved(scratch, chunk, channel_count_);
                    for (std::size_t index = 0; index < chunk; ++index) {
                        const std::uint64_t source_frame =
                            selected_start + static_cast<std::uint64_t>(written + index);
                        const float envelope = adjustment_->envelope_at(source_frame);
                        for (std::size_t channel = 0; channel < channel_count_; ++channel) {
                            scratch[index * channel_count_ + channel] *= envelope;
                        }
                    }
                    output_limiter_->process_interleaved(scratch, chunk, channel_count_);
                    output_guard_->process_interleaved(scratch, chunk, channel_count_);
                    loudness_meter_->process_interleaved(scratch, chunk, channel_count_);
                    const LoudnessSnapshot loudness = loudness_meter_->snapshot();
                    momentary_lufs_.store(loudness.momentary_lufs, std::memory_order_release);
                    output_peak_dbfs_.store(loudness.sample_peak_dbfs, std::memory_order_release);
                    gain_reduction_decibels_.store(
                        dynamics_processor_->gain_reduction_decibels(),
                        std::memory_order_release
                    );
                    limiter_reduction_decibels_.store(
                        output_limiter_->gain_reduction_decibels(),
                        std::memory_order_release
                    );
                    const std::size_t pushed = ring_->write(scratch, chunk);
                    written += pushed;
                }
                if (frame_end >= adjustment_->end_frame()) {
                    av_freep(&output_data[0]);
                    av_frame_unref(frame_.get());
                    return true;
                }
            }
            av_freep(&output_data[0]);
        }
        av_frame_unref(frame_.get());
        return false;
    }

    void producer_loop() {
        std::vector<float> scratch(4096 * channel_count_);
        while (true) {
            {
                std::unique_lock<std::mutex> lock(control_mutex_);
                control_cv_.wait(lock, [this] { return !paused_ || stopped_ || seek_requested_; });
                if (stopped_) {
                    return;
                }
                if (seek_requested_) {
                    const std::uint64_t target = pending_seek_millis_;
                    seek_requested_ = false;
                    lock.unlock();
                    perform_seek(target);
                    // A seek after end-of-stream must restart decoding.
                    ended_.store(false, std::memory_order_release);
                    seek_completed_.store(true, std::memory_order_release);
                    continue;
                }
            }
            if (seek_completed_.load(std::memory_order_acquire)) {
                seek_completed_.store(false, std::memory_order_release);
            }

            if (ended_.load(std::memory_order_acquire)) {
                std::this_thread::sleep_for(std::chrono::milliseconds(2));
                continue;
            }

            const int read_result = av_read_frame(format_.get(), packet_.get());
            if (read_result == AVERROR_EOF) {
                avcodec_send_packet(codec_.get(), nullptr);
                decode_and_feed(scratch.data(), scratch.size() / channel_count_);
                av_packet_unref(packet_.get());
                ended_.store(true, std::memory_order_release);
                continue;
            }
            if (read_result < 0) {
                av_packet_unref(packet_.get());
                continue;
            }
            if (packet_->stream_index == stream_index_) {
                if (avcodec_send_packet(codec_.get(), packet_.get()) == 0) {
                    if (decode_and_feed(scratch.data(), scratch.size() / channel_count_)) {
                        ended_.store(true, std::memory_order_release);
                    }
                }
            }
            av_packet_unref(packet_.get());
        }
    }

    std::string path_;
    FormatContext format_;
    int stream_index_ = 0;
    std::uint64_t duration_millis_ = 0;
    std::uint32_t channel_count_ = 1;
    std::unique_ptr<AVCodecContext, DecoderContextDeleter> codec_;
    std::unique_ptr<SwrContext, SwrDeleter> swr_;
    std::unique_ptr<AVPacket, PacketDeleter> packet_;
    std::unique_ptr<AVFrame, FrameDeleter> frame_;
    std::unique_ptr<FrameRing> ring_;
    std::unique_ptr<PreparedAdjustment> adjustment_;
    std::unique_ptr<LowCutFilter> low_cut_filter_;
    std::unique_ptr<ParametricEqualizer> equalizer_;
    std::unique_ptr<DynamicsProcessor> dynamics_processor_;
    std::unique_ptr<AlgorithmicReverb> reverb_;
    std::unique_ptr<OutputLimiter> output_limiter_;
    std::unique_ptr<OutputGuard> output_guard_;
    std::unique_ptr<LoudnessMeter> loudness_meter_;
    std::uint64_t decoded_frame_cursor_ = 0;
    std::mutex dynamics_mutex_;
    ParametricEqualizerAdjustment pending_equalizer_;
    bool equalizer_update_pending_ = false;
    CompressorAdjustment pending_compressor_;
    bool compressor_update_pending_ = false;
    ReverbAdjustment pending_reverb_;
    bool reverb_update_pending_ = false;
    LimiterAdjustment pending_limiter_;
    bool limiter_update_pending_ = false;
    std::atomic<float> momentary_lufs_{-70.0F};
    std::atomic<float> output_peak_dbfs_{-70.0F};
    std::atomic<float> gain_reduction_decibels_{0.0F};
    std::atomic<float> limiter_reduction_decibels_{0.0F};

    std::thread thread_;
    std::mutex control_mutex_;
    std::condition_variable control_cv_;
    std::atomic<bool> paused_{false};
    std::atomic<bool> stopped_{false};
    bool seek_requested_ = false;
    std::uint64_t pending_seek_millis_ = 0;
    std::atomic<bool> seek_completed_{false};

    std::atomic<bool> ended_{false};
    std::atomic<std::uint64_t> consumed_frames_{0};
};

PlaybackSession::PlaybackSession(const std::string& path, PlaybackAdjustment adjustment) :
    impl_(std::make_unique<Impl>(path, adjustment)) {
    impl_->start();
}

PlaybackSession::~PlaybackSession() = default;

std::size_t PlaybackSession::read(float* output, std::size_t max_frames) {
    return impl_->read(output, max_frames);
}

void PlaybackSession::pause() {
    impl_->pause();
}
void PlaybackSession::resume() {
    impl_->resume();
}
void PlaybackSession::stop() {
    impl_->stop();
}
void PlaybackSession::seek(std::uint64_t millis) {
    impl_->seek(millis);
}
void PlaybackSession::update_equalizer(ParametricEqualizerAdjustment adjustment) {
    impl_->update_equalizer(adjustment);
}
void PlaybackSession::update_compressor(CompressorAdjustment adjustment) {
    impl_->update_compressor(adjustment);
}
void PlaybackSession::update_reverb(ReverbAdjustment adjustment) {
    impl_->update_reverb(adjustment);
}
void PlaybackSession::update_limiter(LimiterAdjustment adjustment) {
    impl_->update_limiter(adjustment);
}

bool PlaybackSession::is_paused() const {
    return impl_->is_paused();
}
bool PlaybackSession::is_stopped() const {
    return impl_->is_stopped();
}
bool PlaybackSession::is_ended() const {
    return impl_->is_ended();
}
std::uint64_t PlaybackSession::position_millis() const {
    return impl_->position_millis();
}
std::uint64_t PlaybackSession::duration_millis() const {
    return impl_->duration_millis();
}
std::uint32_t PlaybackSession::sample_rate() const {
    return impl_->sample_rate();
}
std::uint32_t PlaybackSession::channel_count() const {
    return impl_->channel_count();
}
std::size_t PlaybackSession::buffered_frames() const {
    return impl_->buffered_frames();
}
PlaybackMeterSnapshot PlaybackSession::meter_snapshot() const {
    return impl_->meter_snapshot();
}

} // namespace echo::audio
