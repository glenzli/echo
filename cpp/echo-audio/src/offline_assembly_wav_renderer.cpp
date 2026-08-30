#include "echo/audio/offline_assembly_wav_renderer.hpp"

#include "echo/audio/adjustment.hpp"
#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/output_limiter.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <chrono>
#include <cmath>
#include <cstring>
#include <limits>
#include <memory>
#include <numbers>
#include <ranges>
#include <span>
#include <stdexcept>
#include <thread>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint32_t kSampleRate = 48'000;
constexpr std::uint16_t kChannels = 2;
constexpr std::uint16_t kBitDepth = 24;
constexpr std::size_t kChunkFrames = 4'096;
constexpr std::uint64_t kWavHeaderBytes = 44;
constexpr std::size_t kMaximumTracks = 8;
constexpr std::size_t kMaximumClips = 256;
constexpr std::uint64_t kMaximumDurationMillis = 4ULL * 60ULL * 60ULL * 1'000ULL;

template <typename Integer>
void put_little_endian(std::span<std::byte> destination, std::size_t offset, Integer value) {
    static_assert(std::is_unsigned_v<Integer>);
    for (std::size_t index = 0; index < sizeof(Integer); ++index) {
        destination[offset + index] =
            std::byte{static_cast<unsigned char>((value >> (index * 8U)) & 0xffU)};
    }
}

void put_fourcc(std::span<std::byte> destination, std::size_t offset, const char* value) {
    std::memcpy(destination.data() + offset, value, 4);
}

std::array<std::byte, kWavHeaderBytes> wav_header(std::uint32_t data_size) {
    constexpr std::uint16_t bytes_per_sample = kBitDepth / 8U;
    std::array<std::byte, kWavHeaderBytes> header{};
    put_fourcc(header, 0, "RIFF");
    put_little_endian<std::uint32_t>(header, 4, 36U + data_size);
    put_fourcc(header, 8, "WAVE");
    put_fourcc(header, 12, "fmt ");
    put_little_endian<std::uint32_t>(header, 16, 16U);
    put_little_endian<std::uint16_t>(header, 20, 1U);
    put_little_endian<std::uint16_t>(header, 22, kChannels);
    put_little_endian<std::uint32_t>(header, 24, kSampleRate);
    put_little_endian<std::uint32_t>(header, 28, kSampleRate * kChannels * bytes_per_sample);
    put_little_endian<std::uint16_t>(header, 32, kChannels * bytes_per_sample);
    put_little_endian<std::uint16_t>(header, 34, kBitDepth);
    put_fourcc(header, 36, "data");
    put_little_endian<std::uint32_t>(header, 40, data_size);
    return header;
}

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

void report_progress(const OfflineRenderCallbacks& callbacks, double value) {
    if (callbacks.progress) {
        callbacks.progress(std::clamp(value, 0.0, 1.0));
    }
}

class Dither {
  public:
    float tpdf() {
        return (uniform() - uniform()) / 8'388'608.0F;
    }

  private:
    float uniform() {
        state_ ^= state_ << 13U;
        state_ ^= state_ >> 17U;
        state_ ^= state_ << 5U;
        return static_cast<float>(state_ & 0x00ff'ffffU) / 16'777'216.0F;
    }

    std::uint32_t state_ = 0x4153'4d42U;
};

void encode_pcm24(
    std::span<const float> samples,
    std::vector<std::byte>& encoded,
    std::vector<float>& measured,
    Dither& dither
) {
    constexpr std::int32_t scale = 8'388'607;
    encoded.resize(samples.size() * 3U);
    measured.resize(samples.size());
    for (std::size_t index = 0; index < samples.size(); ++index) {
        const float prepared = std::clamp(
            samples[index] + dither.tpdf(),
            -1.0F,
            static_cast<float>(scale) / 8'388'608.0F
        );
        const auto value = std::clamp(
            static_cast<std::int32_t>(std::lrint(prepared * static_cast<float>(scale))),
            -8'388'608,
            scale
        );
        const auto bits = static_cast<std::uint32_t>(value);
        const std::size_t offset = index * 3U;
        encoded[offset] = std::byte{static_cast<unsigned char>(bits & 0xffU)};
        encoded[offset + 1] = std::byte{static_cast<unsigned char>((bits >> 8U) & 0xffU)};
        encoded[offset + 2] = std::byte{static_cast<unsigned char>((bits >> 16U) & 0xffU)};
        measured[index] = static_cast<float>(value) / static_cast<float>(scale);
    }
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
            if (clip.fade_in_millis + clip.fade_out_millis > duration
                || clip.timeline_start_millis + duration > kMaximumDurationMillis) {
                throw std::invalid_argument("assembly clip duration or fades are invalid");
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
    if (!state.active_track || cursor_frame >= state.timeline_end_frame
        || cursor_frame + frame_count <= state.timeline_start_frame) {
        return;
    }
    const std::uint64_t overlap_start = std::max(cursor_frame, state.timeline_start_frame);
    const std::uint64_t overlap_end =
        std::min(cursor_frame + static_cast<std::uint64_t>(frame_count), state.timeline_end_frame);
    const auto overlap_frames = static_cast<std::size_t>(overlap_end - overlap_start);
    if (!state.session) {
        state.session = std::make_unique<PlaybackSession>(
            state.clip->path,
            PlaybackAdjustment{},
            PlaybackPipelineOptions{.apply_output_guard = false, .collect_metering = false}
        );
        const std::uint64_t clip_offset_frames = overlap_start - state.timeline_start_frame;
        const std::uint64_t source_frame = state.source_start_frame + clip_offset_frames;
        state.session->seek(source_frame * 1'000U / kSampleRate);
    }
    scratch.assign(overlap_frames * kChannels, 0.0F);
    const std::size_t received =
        read_exact(*state.session, scratch.data(), overlap_frames, callbacks);
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
        float left = scratch[frame * kChannels] * state.gain * fade;
        float right = scratch[frame * kChannels + 1] * state.gain * fade;
        apply_pan(left, right, state.clip->pan_percent);
        apply_pan(left, right, state.track->pan_percent);
        mix[(mix_offset + frame) * kChannels] += left;
        mix[(mix_offset + frame) * kChannels + 1] += right;
    }
    if (overlap_end == state.timeline_end_frame) {
        state.session->stop();
        state.session.reset();
    }
}

} // namespace

OfflineRenderResult OfflineAssemblyWavRenderer::render(
    const AssemblyMixPlan& plan,
    RenderByteSink& sink,
    const OfflineRenderCallbacks& callbacks
) {
    if (plan.master_gain_centibels < -2'400 || plan.master_gain_centibels > 1'200
        || plan.limiter_ceiling_centibels < -600 || plan.limiter_ceiling_centibels > 0
        || plan.limiter_release_millis < 20 || plan.limiter_release_millis > 1'000) {
        throw std::invalid_argument("assembly master mix is outside the supported range");
    }
    auto states = prepare_clip_states(plan);
    const std::uint64_t expected_frames =
        std::ranges::max(states | std::views::transform(&ClipState::timeline_end_frame));
    constexpr std::uint64_t bytes_per_frame = kChannels * (kBitDepth / 8U);
    if (expected_frames > (std::numeric_limits<std::uint32_t>::max() - 36U) / bytes_per_frame) {
        throw std::invalid_argument("assembly exceeds the WAV v1 size limit");
    }

    sink.write(wav_header(0));
    OfflineLoudnessAnalyzer analyzer(kSampleRate, kChannels);
    OutputLimiter limiter(
        {.enabled = plan.limiter_enabled,
         .ceiling_centibels = plan.limiter_ceiling_centibels,
         .release_millis = plan.limiter_release_millis},
        kSampleRate
    );
    const float master_gain = gain_amplitude(plan.master_gain_centibels);
    std::vector<float> mix;
    std::vector<float> scratch;
    std::vector<float> measured;
    std::vector<std::byte> encoded;
    Dither dither;
    std::uint64_t cursor = 0;
    std::uint64_t data_bytes = 0;
    while (cursor < expected_frames) {
        if (cancelled(callbacks)) {
            throw OfflineRenderCancelled();
        }
        const auto frames = static_cast<std::size_t>(
            std::min<std::uint64_t>(kChunkFrames, expected_frames - cursor)
        );
        mix.assign(frames * kChannels, 0.0F);
        for (auto& state : states) {
            mix_clip(state, cursor, frames, mix, scratch, callbacks);
        }
        for (float& sample : mix) {
            sample *= master_gain;
        }
        limiter.process_interleaved(mix.data(), frames, kChannels);
        encode_pcm24(mix, encoded, measured, dither);
        sink.write(encoded);
        analyzer.process_interleaved(measured.data(), frames, kChannels);
        cursor += frames;
        data_bytes += encoded.size();
        report_progress(
            callbacks,
            static_cast<double>(cursor) / static_cast<double>(expected_frames)
        );
    }
    sink.seek(0);
    sink.write(wav_header(static_cast<std::uint32_t>(data_bytes)));
    report_progress(callbacks, 1.0);
    const auto loudness = analyzer.result();
    return {
        .frame_count = expected_frames,
        .size_bytes = kWavHeaderBytes + data_bytes,
        .sample_rate = kSampleRate,
        .channel_count = kChannels,
        .bit_depth = kBitDepth,
        .integrated_lufs = loudness.integrated_lufs,
        .true_peak_dbtp = loudness.true_peak_dbtp,
    };
}

} // namespace echo::audio
