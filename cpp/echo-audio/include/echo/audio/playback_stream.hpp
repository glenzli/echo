#pragma once
#include <cstddef>
#include <cstdint>
#include <string>

namespace echo::audio {
/// Lock-free producer snapshot for desktop metering. Values describe the
/// prepared signal before device volume, never the immutable original.
struct PlaybackMeterSnapshot {
    float momentary_lufs = -70.0F;
    float output_peak_dbfs = -70.0F;
    float gain_reduction_decibels = 0.0F;
    float limiter_reduction_decibels = 0.0F;
};

// Transport shared by source DSP and a prepared multitrack mix. Only read()
// enters the device callback; all ownership and control remain off that thread.
class PlaybackStream {
  public:
    virtual ~PlaybackStream() = default;
    virtual std::size_t read(float* output, std::size_t max_frames) = 0;
    virtual void pause() = 0;
    virtual void resume() = 0;
    virtual void stop() = 0;
    virtual void seek(std::uint64_t millis) = 0;
    virtual bool is_paused() const = 0;
    virtual bool is_stopped() const = 0;
    virtual bool is_ended() const = 0;
    virtual std::uint64_t position_millis() const = 0;
    virtual std::uint64_t duration_millis() const = 0;
    virtual std::uint64_t output_frame_count() const = 0;
    virtual std::uint32_t sample_rate() const = 0;
    virtual std::uint32_t channel_count() const = 0;
    virtual std::size_t buffered_frames() const = 0;
    virtual PlaybackMeterSnapshot meter_snapshot() const = 0;
    virtual std::string error() const {
        return {};
    }
};
} // namespace echo::audio
