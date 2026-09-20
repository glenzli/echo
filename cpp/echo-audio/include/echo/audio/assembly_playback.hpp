#pragma once
#include "echo/audio/assembly.hpp"
#include "echo/audio/playback_stream.hpp"
#include <memory>

namespace echo::audio {
// A producer mixes a bounded amount ahead; the callback only copies its ring.
// No complete mixed WAV is prepared, and buffering is independent of duration.
class AssemblyPlaybackSession final : public PlaybackStream {
  public:
    explicit AssemblyPlaybackSession(AssemblyMixPlan plan, bool apply_output_guard = true);
    ~AssemblyPlaybackSession() override;
    AssemblyPlaybackSession(const AssemblyPlaybackSession&) = delete;
    AssemblyPlaybackSession& operator=(const AssemblyPlaybackSession&) = delete;
    std::size_t read(float* output, std::size_t max_frames) override;
    void pause() override;
    void resume() override;
    void stop() override;
    void seek(std::uint64_t millis) override;
    bool is_paused() const override;
    bool is_stopped() const override;
    bool is_ended() const override;
    std::uint64_t position_millis() const override;
    std::uint64_t duration_millis() const override;
    std::uint64_t output_frame_count() const override;
    std::uint32_t sample_rate() const override;
    std::uint32_t channel_count() const override;
    std::size_t buffered_frames() const override;
    PlaybackMeterSnapshot meter_snapshot() const override;
    std::string error() const override;
    static constexpr std::size_t capacity_frames = 16384;

  private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};
} // namespace echo::audio
