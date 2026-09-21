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
    std::size_t track_index = 0;
    std::uint64_t timeline_start_frame = 0;
    std::uint64_t timeline_end_frame = 0;
    std::uint64_t source_start_frame = 0;
    std::uint64_t fade_in_frames = 0;
    std::uint64_t fade_out_frames = 0;
    float gain = 1.0F;
    std::unique_ptr<PreparedPcmReader> prepared;
    std::unique_ptr<PlaybackSession> session;
};

std::vector<ClipState> prepare_clip_states(const AssemblyMixPlan& plan) {
    if (plan.tracks.empty() || plan.tracks.size() > kMaximumTracks) {
        throw std::invalid_argument("assembly requires between one and eight tracks");
    }
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
            state.track_index = static_cast<std::size_t>(&track - plan.tracks.data());
            state.timeline_start_frame = frames_from_millis(clip.timeline_start_millis);
            state.timeline_end_frame = state.timeline_start_frame + frames_from_millis(duration);
            state.source_start_frame = frames_from_millis(clip.source_start_millis);
            state.fade_in_frames = frames_from_millis(clip.fade_in_millis);
            state.fade_out_frames = frames_from_millis(clip.fade_out_millis);
            state.gain = gain_amplitude(clip.gain_centibels);
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
    if (state.clip->muted || cursor_frame >= state.timeline_end_frame
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
        mix[(mix_offset + frame) * kChannels] += left;
        mix[(mix_offset + frame) * kChannels + 1] += right;
    }
    if (overlap_end == state.timeline_end_frame) {
        state.session.reset();
        state.prepared.reset();
    }
}

struct MixRamp {
    float current = 1, target = 1;
    std::size_t remaining = 0;
    void set(float value) {
        if (target == value)
            return;
        target = value;
        remaining = 480;
    }
    float next() {
        if (remaining) {
            current += (target - current) / static_cast<float>(remaining--);
        }
        return current;
    }
    void reset() {
        current = target;
        remaining = 0;
    }
};

} // namespace
AssemblyMixControls assembly_mix_controls(const AssemblyMixPlan& plan) {
    AssemblyMixControls controls;
    controls.track_count = plan.tracks.size();
    controls.master_gain_centibels = plan.master_gain_centibels;
    for (std::size_t i = 0; i < std::min(plan.tracks.size(), controls.tracks.size()); ++i) {
        const auto& track = plan.tracks[i];
        controls.tracks[i] = {track.gain_centibels, track.pan_percent, track.muted, track.solo};
    }
    return controls;
}
bool valid_assembly_mix_controls(const AssemblyMixControls& controls, std::size_t track_count) {
    if (track_count == 0 || track_count > controls.tracks.size()
        || controls.track_count != track_count || controls.master_gain_centibels < -2400
        || controls.master_gain_centibels > 1200)
        return false;
    for (std::size_t i = 0; i < track_count; ++i) {
        const auto& track = controls.tracks[i];
        if (track.gain_centibels < -2400 || track.gain_centibels > 1200 || track.pan_percent < -100
            || track.pan_percent > 100)
            return false;
    }
    return true;
}
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
        track_mix_.reserve(AssemblyMixer::block_frames * kChannels);
        update_mix(assembly_mix_controls(plan_));
        reset_ramps();
    }
    bool update_mix(const AssemblyMixControls& controls) {
        if (!valid_assembly_mix_controls(controls, plan_.tracks.size()))
            return false;
        bool solo = false;
        for (std::size_t i = 0; i < controls.track_count; ++i)
            solo = solo || controls.tracks[i].solo;
        for (std::size_t i = 0; i < controls.track_count; ++i) {
            const auto& track = controls.tracks[i];
            float left =
                !track.muted && (!solo || track.solo) ? gain_amplitude(track.gain_centibels) : 0;
            float right = left;
            apply_pan(left, right, track.pan_percent);
            left_[i].set(left);
            right_[i].set(right);
        }
        master_.set(gain_amplitude(controls.master_gain_centibels));
        return true;
    }
    void reset_ramps() {
        for (auto& ramp : left_)
            ramp.reset();
        for (auto& ramp : right_)
            ramp.reset();
        master_.reset();
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
        peaks_ = {};
        for (std::size_t track = 0; track < plan_.tracks.size(); ++track) {
            const bool silent = left_[track].current == 0 && left_[track].target == 0
                                && right_[track].current == 0 && right_[track].target == 0;
            track_mix_.assign(frames * kChannels, 0.0F);
            for (auto& state : states_) {
                if (state.track_index != track)
                    continue;
                if (silent) {
                    // Reopening at the current source offset prevents stale audio on unmute.
                    state.prepared.reset();
                    state.session.reset();
                } else {
                    mix_clip(state, cursor_, frames, track_mix_, scratch_, callbacks);
                }
            }
            float left_peak = 0, right_peak = 0;
            for (std::size_t frame = 0; frame < frames; ++frame) {
                const float left = track_mix_[frame * 2] * left_[track].next();
                const float right = track_mix_[frame * 2 + 1] * right_[track].next();
                mix_[frame * 2] += left;
                mix_[frame * 2 + 1] += right;
                left_peak = std::max(left_peak, std::abs(left));
                right_peak = std::max(right_peak, std::abs(right));
            }
            peaks_[track] = {
                20 * std::log10(std::max(left_peak, 0.00031622777F)),
                20 * std::log10(std::max(right_peak, 0.00031622777F))
            };
        }
        for (std::size_t frame = 0; frame < frames; ++frame) {
            const float gain = master_.next();
            mix_[frame * 2] *= gain;
            mix_[frame * 2 + 1] *= gain;
        }
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
        reset_ramps();
        peaks_ = {};
    }
    AssemblyMixPlan plan_;
    std::vector<ClipState> states_;
    OutputLimiter limiter_;
    std::uint64_t start_ = 0, end_ = 0, cursor_ = 0;
    std::vector<float> mix_, scratch_, track_mix_;
    std::array<MixRamp, 8> left_, right_;
    MixRamp master_;
    AssemblyTrackPeaks peaks_;
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
bool AssemblyMixer::update_mix(const AssemblyMixControls& controls) {
    return impl_->update_mix(controls);
}
AssemblyTrackPeaks AssemblyMixer::track_peaks() const {
    return impl_->peaks_;
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
