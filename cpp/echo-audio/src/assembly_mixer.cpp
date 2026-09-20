#include "echo/audio/assembly_mixer.hpp"
#include "echo/audio/output_limiter.hpp"
#include "echo/audio/playback.hpp"
#include "prepared_pcm_reader.hpp"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <limits>
#include <numbers>
#include <ranges>
#include <thread>
#include <utility>
#include <vector>

namespace echo::audio {
namespace {
constexpr std::uint32_t kSampleRate = 48000;
constexpr std::uint16_t kChannels = 2;
constexpr std::size_t kMaximumTracks = 8;
constexpr std::size_t kMaximumClips = 256;
constexpr std::uint64_t kMaximumDurationMillis = 14400000;
std::uint64_t frames_from_millis(std::uint64_t millis) {
    if (millis > std::numeric_limits<std::uint64_t>::max() / kSampleRate) {
        throw std::invalid_argument("assembly time exceeds the supported range");
    }
    return millis * kSampleRate / 1'000U;
}

float gain_amplitude(std::int16_t centibels) {
    return std::pow(10.0F, static_cast<float>(centibels) / 2'000.0F);
}

float fade_amplitude(AssemblyFadeCurve curve, float progress) {
    const float x = std::clamp(progress, 0.0F, 1.0F);
    switch (curve) {
    case AssemblyFadeCurve::Linear:
        return x;
    case AssemblyFadeCurve::Smooth:
        return x * x * (3.0F - 2.0F * x);
    case AssemblyFadeCurve::EqualPower:
        return std::sin(x * std::numbers::pi_v<float> * 0.5F);
    }
    throw std::invalid_argument("assembly fade curve is invalid");
}

void apply_pan(float& left, float& right, std::int16_t pan_percent) {
    const float pan = static_cast<float>(pan_percent) / 100.0F;
    if (pan < 0.0F) {
        right *= 1.0F + pan;
    } else {
        left *= 1.0F - pan;
    }
}

bool cancelled(const OfflineRenderCallbacks& callbacks) {
    return callbacks.cancelled && callbacks.cancelled();
}

struct ClipState {
    const AssemblyClipSource* clip = nullptr;
    const AssemblyTrackMix* track = nullptr;
    std::uint64_t timeline_start_frame = 0;
    std::uint64_t timeline_end_frame = 0;
    std::uint64_t source_start_frame = 0;
    std::uint64_t fade_in_frames = 0;
    std::uint64_t fade_out_frames = 0;
    float gain = 1.0F;
    bool active_track = true;
    std::unique_ptr<PreparedPcmReader> prepared;
    std::unique_ptr<PlaybackSession> session;
};

std::vector<ClipState> prepare_clip_states(const AssemblyMixPlan& plan) {
    if (plan.tracks.empty() || plan.tracks.size() > kMaximumTracks) {
        throw std::invalid_argument("assembly requires between one and eight tracks");
    }
    const bool has_solo = std::ranges::any_of(plan.tracks, &AssemblyTrackMix::solo);
    std::vector<ClipState> states;
    for (const auto& track : plan.tracks) {
        if (track.gain_centibels < -2'400 || track.gain_centibels > 1'200
            || track.pan_percent < -100 || track.pan_percent > 100) {
            throw std::invalid_argument("assembly track mix is outside the supported range");
        }
        for (const auto& clip : track.clips) {
            if (clip.path.empty() || clip.source_end_millis <= clip.source_start_millis
                || clip.gain_centibels < -2'400 || clip.gain_centibels > 1'200
                || clip.pan_percent < -100 || clip.pan_percent > 100) {
                throw std::invalid_argument("assembly clip is invalid");
            }
            const std::uint64_t duration = clip.source_end_millis - clip.source_start_millis;
            if (duration > kMaximumDurationMillis || clip.fade_in_millis > duration
                || clip.fade_out_millis > duration - clip.fade_in_millis
                || clip.timeline_start_millis > kMaximumDurationMillis - duration) {
                throw std::invalid_argument("assembly clip duration or fades are invalid");
            }
            const auto& points = clip.gain_envelope;
            if (points.size() > 2048)
                throw std::invalid_argument("too many gain envelope points");
            for (std::size_t i = 0; i < points.size(); ++i) {
                if (points[i].gain_centibels < -9600 || points[i].gain_centibels > 1200
                    || points[i].source_millis
                           > std::numeric_limits<std::uint64_t>::max() / kSampleRate
                    || (i > 0 && points[i - 1].source_millis >= points[i].source_millis))
                    throw std::invalid_argument("invalid gain envelope point");
            }
            ClipState state;
            state.clip = &clip;
            state.track = &track;
            state.timeline_start_frame = frames_from_millis(clip.timeline_start_millis);
            state.timeline_end_frame = state.timeline_start_frame + frames_from_millis(duration);
            state.source_start_frame = frames_from_millis(clip.source_start_millis);
            state.fade_in_frames = frames_from_millis(clip.fade_in_millis);
            state.fade_out_frames = frames_from_millis(clip.fade_out_millis);
            state.gain = gain_amplitude(clip.gain_centibels) * gain_amplitude(track.gain_centibels);
            state.active_track = !track.muted && (!has_solo || track.solo) && !clip.muted;
            states.push_back(std::move(state));
        }
    }
    if (states.empty() || states.size() > kMaximumClips) {
        throw std::invalid_argument("assembly requires between one and 256 clips");
    }
    return states;
}

std::size_t read_exact(
    PlaybackSession& session,
    float* output,
    std::size_t frames,
    const OfflineRenderCallbacks& callbacks
) {
    std::size_t received = 0;
    while (received < frames) {
        if (cancelled(callbacks)) {
            session.stop();
            throw OfflineRenderCancelled();
        }
        const std::size_t read = session.read(output + received * kChannels, frames - received);
        if (read == 0) {
            if (session.is_ended() && session.buffered_frames() == 0) {
                break;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            continue;
        }
        received += read;
    }
    return received;
}

void mix_clip(
    ClipState& state,
    std::uint64_t cursor_frame,
    std::size_t frame_count,
    std::vector<float>& mix,
    std::vector<float>& scratch,
    const OfflineRenderCallbacks& callbacks
) {
    if (cancelled(callbacks))
        throw OfflineRenderCancelled();
    if (!state.active_track || cursor_frame >= state.timeline_end_frame
        || cursor_frame + frame_count <= state.timeline_start_frame) {
        return;
    }
    const std::uint64_t overlap_start = std::max(cursor_frame, state.timeline_start_frame);
    const std::uint64_t overlap_end =
        std::min(cursor_frame + static_cast<std::uint64_t>(frame_count), state.timeline_end_frame);
    const auto overlap_frames = static_cast<std::size_t>(overlap_end - overlap_start);
    if (!state.prepared && !state.session) {
        const std::uint64_t clip_offset_frames = overlap_start - state.timeline_start_frame;
        const std::uint64_t source_frame = state.source_start_frame + clip_offset_frames;
        state.prepared = PreparedPcmReader::open(state.clip->path);
        if (state.prepared) {
            state.prepared->seek(source_frame);
        } else {
            state.session = std::make_unique<PlaybackSession>(
                state.clip->path,
                PlaybackAdjustment{},
                PlaybackPipelineOptions{.apply_output_guard = false, .collect_metering = false}
            );
            state.session->seek(source_frame * 1'000U / kSampleRate);
        }
    }
    scratch.assign(overlap_frames * kChannels, 0.0F);
    const std::size_t received =
        state.prepared ? state.prepared->read(scratch)
                       : read_exact(*state.session, scratch.data(), overlap_frames, callbacks);
    if (received != overlap_frames) {
        throw std::runtime_error("prepared assembly source ended before its clip range");
    }
    const std::size_t mix_offset = static_cast<std::size_t>(overlap_start - cursor_frame);
    const std::uint64_t clip_duration = state.timeline_end_frame - state.timeline_start_frame;
    for (std::size_t frame = 0; frame < overlap_frames; ++frame) {
        const std::uint64_t clip_frame = overlap_start - state.timeline_start_frame + frame;
        float fade = 1.0F;
        if (state.fade_in_frames > 0 && clip_frame < state.fade_in_frames) {
            fade *= fade_amplitude(
                state.clip->fade_in_curve,
                static_cast<float>(clip_frame) / static_cast<float>(state.fade_in_frames)
            );
        }
        const std::uint64_t remaining = clip_duration - clip_frame;
        if (state.fade_out_frames > 0 && remaining <= state.fade_out_frames) {
            fade *= fade_amplitude(
                state.clip->fade_out_curve,
                static_cast<float>(remaining) / static_cast<float>(state.fade_out_frames)
            );
        }
        if (state.clip->gain_envelope_enabled && !state.clip->gain_envelope.empty()) {
            const auto& points = state.clip->gain_envelope;
            const double source_millis =
                static_cast<double>(state.source_start_frame + clip_frame) / 48.0;
            const auto upper = std::upper_bound(
                points.begin(),
                points.end(),
                source_millis,
                [](double time, const AssemblyGainPoint& point) {
                    return time < static_cast<double>(point.source_millis);
                }
            );
            double gain = 0;
            if (upper == points.begin())
                gain = points.front().gain_centibels;
            else if (upper == points.end())
                gain = points.back().gain_centibels;
            else {
                const auto& a = *(upper - 1);
                const auto& b = *upper;
                const double t = (source_millis - static_cast<double>(a.source_millis))
                                 / static_cast<double>(b.source_millis - a.source_millis);
                gain = a.gain_centibels + t * (b.gain_centibels - a.gain_centibels);
            }
            fade *= static_cast<float>(std::pow(10.0, gain / 2000.0));
        }
        float left = scratch[frame * kChannels] * state.gain * fade;
        float right = scratch[frame * kChannels + 1] * state.gain * fade;
        apply_pan(left, right, state.clip->pan_percent);
        apply_pan(left, right, state.track->pan_percent);
        mix[(mix_offset + frame) * kChannels] += left;
        mix[(mix_offset + frame) * kChannels + 1] += right;
    }
    if (overlap_end == state.timeline_end_frame) {
        state.session.reset();
        state.prepared.reset();
    }
}

} // namespace
class AssemblyMixer::Impl {
  public:
    explicit Impl(AssemblyMixPlan value) :
        plan_(std::move(value)), limiter_(
                                     {.enabled = plan_.limiter_enabled,
                                      .ceiling_centibels = plan_.limiter_ceiling_centibels,
                                      .release_millis = plan_.limiter_release_millis},
                                     kSampleRate
                                 ) {
        const auto& plan = plan_;
        if (plan.master_gain_centibels < -2'400 || plan.master_gain_centibels > 1'200
            || plan.limiter_ceiling_centibels < -600 || plan.limiter_ceiling_centibels > 0
            || plan.limiter_release_millis < 20 || plan.limiter_release_millis > 1'000) {
            throw std::invalid_argument("assembly master mix is outside the supported range");
        }
        states_ = prepare_clip_states(plan);
        const std::uint64_t project_frames =
            std::ranges::max(states_ | std::views::transform(&ClipState::timeline_end_frame));
        if (plan.render_start_millis > kMaximumDurationMillis
            || plan.render_end_millis > kMaximumDurationMillis)
            throw std::invalid_argument("assembly preview window is outside the supported range");
        start_ = frames_from_millis(plan.render_start_millis);
        end_ = plan.render_end_millis == 0
                   ? project_frames
                   : std::min(project_frames, frames_from_millis(plan.render_end_millis));
        if (start_ >= end_)
            throw std::invalid_argument("assembly preview window is empty");
        cursor_ = start_;

        mix_.reserve(AssemblyMixer::block_frames * kChannels);
        scratch_.reserve(AssemblyMixer::block_frames * kChannels);
    }
    std::span<const float> next(const OfflineRenderCallbacks& callbacks) {
        if (cancelled(callbacks))
            throw OfflineRenderCancelled();
        if (cursor_ == end_)
            return {};
        const auto frames = static_cast<std::size_t>(
            std::min<std::uint64_t>(AssemblyMixer::block_frames, end_ - cursor_)
        );
        mix_.assign(frames * kChannels, 0.0F);
        for (auto& state : states_)
            mix_clip(state, cursor_, frames, mix_, scratch_, callbacks);
        const float gain = gain_amplitude(plan_.master_gain_centibels);
        for (auto& sample : mix_)
            sample *= gain;
        limiter_.process_interleaved(mix_.data(), frames, kChannels);
        cursor_ += frames;
        return mix_;
    }
    void seek(std::uint64_t millis) {
        const auto bounded = std::min(millis, (end_ - start_) / 48);
        cursor_ = start_ + bounded * 48;
        for (auto& state : states_) {
            state.session.reset();
            state.prepared.reset();
        }
        limiter_.reset();
    }
    AssemblyMixPlan plan_;
    std::vector<ClipState> states_;
    OutputLimiter limiter_;
    std::uint64_t start_ = 0, end_ = 0, cursor_ = 0;
    std::vector<float> mix_, scratch_;
};
AssemblyMixer::AssemblyMixer(AssemblyMixPlan plan) :
    impl_(std::make_unique<Impl>(std::move(plan))) {}
AssemblyMixer::~AssemblyMixer() = default;
std::span<const float> AssemblyMixer::next(const OfflineRenderCallbacks& callbacks) {
    return impl_->next(callbacks);
}
void AssemblyMixer::seek(std::uint64_t millis) {
    impl_->seek(millis);
}
std::uint64_t AssemblyMixer::frame_count() const {
    return impl_->end_ - impl_->start_;
}
std::uint64_t AssemblyMixer::position_frames() const {
    return impl_->cursor_ - impl_->start_;
}
float AssemblyMixer::limiter_reduction_decibels() const {
    return impl_->limiter_.gain_reduction_decibels();
}
} // namespace echo::audio
