#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Lock-free producer snapshot for desktop metering. Values describe the
/// prepared signal before device volume, never the immutable original.
struct PlaybackMeterSnapshot {
    float momentary_lufs = -70.0F;
    float output_peak_dbfs = -70.0F;
    float gain_reduction_decibels = 0.0F;
    float limiter_reduction_decibels = 0.0F;
};

/// Selects preview-only safety stages without changing the authored graph.
/// Offline render disables these stages so the published file contains only
/// the user's adjustments; device playback keeps both enabled by default.
struct PlaybackPipelineOptions {
    bool apply_output_guard = true;
    bool collect_metering = true;
};

/// Streaming playback session over one source.
///
/// The realtime contract: a producer thread decodes the source to the
/// canonical interleaved float mixdown (48 kHz, channel-preserving) into a
/// fixed SPSC ring; `read()` is the only method the audio callback touches
/// and it performs no allocation, no FFmpeg calls, and no locking. All
/// control commands (pause/resume/seek/stop) are non-realtime and are
/// handled by the producer thread.
class PlaybackSession {
  public:
    /// Opens (and begins decoding immediately) the source.
    ///
    /// @throws std::runtime_error when the source cannot be opened.
    explicit PlaybackSession(
        const std::string& path,
        PlaybackAdjustment adjustment = PlaybackAdjustment{},
        PlaybackPipelineOptions options = PlaybackPipelineOptions{}
    );
    ~PlaybackSession();

    PlaybackSession(const PlaybackSession&) = delete;
    PlaybackSession& operator=(const PlaybackSession&) = delete;

    /// Audio-callback safe: copies up to `max_frames` interleaved frames into
    /// `output`, returning the number copied. Returns 0 when paused (after
    /// the ring drains), after stop, or at end of stream.
    std::size_t read(float* output, std::size_t max_frames);

    void pause();
    void resume();
    void stop();
    /// Seeks to `millis`; the producer performs the reposition at the next
    /// packet boundary and drains the ring first.
    void seek(std::uint64_t millis);
    /// Publishes a new EQ target without replacing the decoder or playback
    /// session. The producer coalesces rapid updates and crossfades filter
    /// state before the audio reaches the realtime ring.
    void update_equalizer(ParametricEqualizerAdjustment adjustment);
    /// Publishes latest-wins restoration targets without restarting playback.
    void update_restoration(RestorationAdjustment adjustment);
    /// Publishes a click-free power-line hum target without restarting.
    void update_de_hum(DeHumAdjustment adjustment);
    /// Publishes a latest-wins short-transient repair target. The node's
    /// fixed latency remains stable for the lifetime of the session.
    void update_de_click(DeClickAdjustment adjustment);
    /// Publishes a latest-wins compressor target. The producer applies it to
    /// the persistent detector without restarting playback.
    void update_compressor(CompressorAdjustment adjustment);
    /// Publishes a click-free algorithmic room target without restarting.
    void update_reverb(ReverbAdjustment adjustment);
    /// Publishes a latest-wins limiter target without restarting playback.
    void update_limiter(LimiterAdjustment adjustment);

    [[nodiscard]] bool is_paused() const;
    [[nodiscard]] bool is_stopped() const;
    [[nodiscard]] bool is_ended() const;
    [[nodiscard]] std::uint64_t position_millis() const;
    [[nodiscard]] std::uint64_t duration_millis() const;
    /// Exact arranged output frame count after hidden intervals and gaps.
    [[nodiscard]] std::uint64_t output_frame_count() const;
    [[nodiscard]] std::uint32_t sample_rate() const;
    [[nodiscard]] std::uint32_t channel_count() const;
    /// Frames currently buffered and ready for the audio callback.
    [[nodiscard]] std::size_t buffered_frames() const;
    [[nodiscard]] PlaybackMeterSnapshot meter_snapshot() const;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace echo::audio
