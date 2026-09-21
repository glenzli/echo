#include "echo/audio/assembly_playback.hpp"
#include "echo/audio/assembly_mixer.hpp"
#include "echo/audio/loudness_meter.hpp"
#include "echo/audio/output_guard.hpp"
#include "playback_frame_ring.hpp"
#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <mutex>
#include <optional>
#include <thread>
#include <utility>

namespace echo::audio {
namespace {
constexpr std::uint64_t kClosed = std::uint64_t{1} << 63;
constexpr std::uint64_t kReaders = ~kClosed;
} // namespace
class AssemblyPlaybackSession::Impl {
  public:
    Impl(AssemblyMixPlan plan, bool guard) :
        track_count_(plan.tracks.size()), mixer_(std::move(plan)), frames_(mixer_.frame_count()),
        ring_(capacity_frames, 2), guard_(48000), meter_(48000, 2), use_guard_(guard) {
        thread_ = std::jthread([this] { produce(); });
    }
    ~Impl() {
        stop();
    }
    std::size_t read(float* output, std::size_t budget) {
        if (stopped_.load() || failed_.load() || paused_.load())
            return 0;
        auto gate = gate_.load();
        while ((gate & kClosed) == 0) {
            if ((gate & kReaders) == kReaders)
                return 0;
            if (gate_.compare_exchange_weak(gate, gate + 1))
                break;
        }
        if (gate & kClosed)
            return 0;
        std::uint64_t last = kNoSourceFrame;
        const auto count = ring_.read(output, budget, &last);
        if (last != kNoSourceFrame)
            position_.store(last + 1);
        if (gate_.fetch_sub(1) & kClosed) {
            std::fill_n(output, count * 2, 0.0F);
            return 0;
        }
        return count;
    }
    void stop() {
        {
            std::lock_guard lock(control_);
            stopped_.store(true);
        }
        wake_.notify_one();
        if (thread_.joinable())
            thread_.join();
    }
    void seek(std::uint64_t millis) {
        std::lock_guard lock(control_);
        gate_.fetch_or(kClosed);
        pending_seek_ = std::min(millis, frames_ / 48);
        has_seek_ = true;
        requested_.fetch_add(1);
        wake_.notify_one();
    }
    bool update_mix(const AssemblyMixControls& controls) {
        if (!valid_assembly_mix_controls(controls, track_count_))
            return false;
        std::lock_guard lock(control_);
        if (stopped_.load() || failed_.load())
            return false;
        pending_mix_ = controls;
        wake_.notify_one();
        return true;
    }
    void apply_mix() {
        std::optional<AssemblyMixControls> next;
        {
            std::lock_guard lock(control_);
            next = std::exchange(pending_mix_, std::nullopt);
        }
        if (next)
            mixer_.update_mix(*next);
    }
    bool reposition() {
        std::uint64_t target = 0, generation = 0;
        {
            std::lock_guard lock(control_);
            if (!has_seek_)
                return false;
            target = pending_seek_;
            generation = requested_.load();
            has_seek_ = false;
        }
        // The callback is excluded before resetting either ring index.
        while ((gate_.load() & kReaders) != 0 && !stopped_.load())
            std::this_thread::yield();
        if (stopped_.load())
            return true;
        ring_.reset();
        mixer_.seek(target);
        guard_.reset();
        meter_.reset();
        std::lock_guard lock(control_);
        if (requested_.load() == generation) {
            position_.store(target * 48);
            ended_.store(false);
            momentary_.store(-70);
            peak_.store(-70);
            reduction_.store(0);
            gate_.store(0);
        }
        return true;
    }
    void produce() {
        try {
            std::array<float, AssemblyMixer::block_frames * 2> output{};
            std::array<std::uint64_t, AssemblyMixer::block_frames> positions{};
            while (!stopped_.load()) {
                apply_mix();
                if (reposition())
                    continue;
                if (paused_.load() || ended_.load()) {
                    std::unique_lock lock(control_);
                    wake_.wait(lock, [this] {
                        return stopped_.load() || has_seek_ || pending_mix_.has_value()
                               || (!paused_.load() && !ended_.load());
                    });
                    continue;
                }
                if (ring_.writable() < AssemblyMixer::block_frames) {
                    std::unique_lock lock(control_);
                    wake_.wait_for(lock, std::chrono::milliseconds(2), [this] {
                        return stopped_.load() || has_seek_ || pending_mix_.has_value();
                    });
                    continue;
                }
                const auto generation = requested_.load();
                const auto first = mixer_.position_frames();
                std::span<const float> block;
                try {
                    block = mixer_.next({.cancelled = [this, generation] {
                        return stopped_.load() || requested_.load() != generation;
                    }});
                } catch (const OfflineRenderCancelled&) {
                    continue;
                }
                if (stopped_.load() || requested_.load() != generation)
                    continue;
                if (block.empty()) {
                    ended_.store(true);
                    continue;
                }
                const auto count = block.size() / 2;
                std::copy(block.begin(), block.end(), output.begin());
                if (use_guard_)
                    guard_.process_interleaved(output.data(), count, 2);
                meter_.process_interleaved(output.data(), count, 2);
                const auto meter = meter_.snapshot();
                momentary_.store(meter.momentary_lufs);
                peak_.store(meter.sample_peak_dbfs);
                reduction_.store(mixer_.limiter_reduction_decibels());
                const auto peaks = mixer_.track_peaks();
                for (std::size_t i = 0; i < track_count_; ++i) {
                    track_peaks_[i].left.store(peaks[i].left_dbfs);
                    track_peaks_[i].right.store(peaks[i].right_dbfs);
                }
                for (std::size_t i = 0; i < count; ++i)
                    positions[i] = first + i;
                // A concurrent seek closes reads immediately. Its producer-side
                // reset discards this block before reopening the gate.
                ring_.write(output.data(), positions.data(), count);
            }
        } catch (const std::exception& error) {
            std::lock_guard lock(control_);
            error_ = error.what();
            failed_.store(true);
            ended_.store(true);
        }
    }
    const std::size_t track_count_;
    AssemblyMixer mixer_;
    const std::uint64_t frames_;
    PlaybackFrameRing ring_;
    OutputGuard guard_;
    LoudnessMeter meter_;
    bool use_guard_;
    std::atomic<bool> stopped_{false}, paused_{false}, ended_{false}, failed_{false};
    std::atomic<std::uint64_t> gate_{0}, position_{0}, requested_{0};
    std::atomic<float> momentary_{-70}, peak_{-70}, reduction_{0};
    mutable std::mutex control_;
    std::condition_variable wake_;
    bool has_seek_ = false;
    std::uint64_t pending_seek_ = 0;
    std::optional<AssemblyMixControls> pending_mix_;
    struct AtomicPeak {
        std::atomic<float> left{-70}, right{-70};
    };
    std::array<AtomicPeak, 8> track_peaks_;
    std::string error_;
    std::jthread thread_;
};
AssemblyPlaybackSession::AssemblyPlaybackSession(AssemblyMixPlan plan, bool guard) :
    impl_(std::make_unique<Impl>(std::move(plan), guard)) {}
AssemblyPlaybackSession::~AssemblyPlaybackSession() = default;
std::size_t AssemblyPlaybackSession::read(float* output, std::size_t budget) {
    return impl_->read(output, budget);
}
void AssemblyPlaybackSession::pause() {
    std::lock_guard lock(impl_->control_);
    impl_->paused_.store(true);
}
void AssemblyPlaybackSession::resume() {
    std::lock_guard lock(impl_->control_);
    impl_->paused_.store(false);
    impl_->wake_.notify_one();
}
void AssemblyPlaybackSession::stop() {
    impl_->stop();
}
void AssemblyPlaybackSession::seek(std::uint64_t millis) {
    impl_->seek(millis);
}
bool AssemblyPlaybackSession::update_mix(const AssemblyMixControls& controls) {
    return impl_->update_mix(controls);
}
AssemblyTrackPeaks AssemblyPlaybackSession::track_peaks() const {
    AssemblyTrackPeaks result;
    for (std::size_t i = 0; i < impl_->track_count_; ++i)
        result[i] = {impl_->track_peaks_[i].left.load(), impl_->track_peaks_[i].right.load()};
    return result;
}
std::size_t AssemblyPlaybackSession::track_count() const {
    return impl_->track_count_;
}
bool AssemblyPlaybackSession::is_paused() const {
    return impl_->paused_.load();
}
bool AssemblyPlaybackSession::is_stopped() const {
    return impl_->stopped_.load();
}
bool AssemblyPlaybackSession::is_ended() const {
    return impl_->ended_.load();
}
std::uint64_t AssemblyPlaybackSession::position_millis() const {
    return impl_->position_.load() / 48;
}
std::uint64_t AssemblyPlaybackSession::duration_millis() const {
    return impl_->frames_ / 48;
}
std::uint64_t AssemblyPlaybackSession::output_frame_count() const {
    return impl_->frames_;
}
std::uint32_t AssemblyPlaybackSession::sample_rate() const {
    return 48000;
}
std::uint32_t AssemblyPlaybackSession::channel_count() const {
    return 2;
}
std::size_t AssemblyPlaybackSession::buffered_frames() const {
    return impl_->ring_.available();
}
PlaybackMeterSnapshot AssemblyPlaybackSession::meter_snapshot() const {
    return {
        .momentary_lufs = impl_->momentary_.load(),
        .output_peak_dbfs = impl_->peak_.load(),
        .limiter_reduction_decibels = impl_->reduction_.load()
    };
}
std::string AssemblyPlaybackSession::error() const {
    std::lock_guard lock(impl_->control_);
    return impl_->error_;
}
} // namespace echo::audio
