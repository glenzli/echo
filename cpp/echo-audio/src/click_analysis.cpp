#include "echo/audio/click_analysis.hpp"
#include "echo/audio/decode.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <stdexcept>
#include <thread>

namespace echo::audio {
namespace {
constexpr std::size_t kBlockFrames = 4096;
constexpr std::size_t kMaximumCandidates = 256;
constexpr std::uint64_t kFramesPerMilli = 48;
constexpr float kDifferenceFloor = 0.00001F;

DeClickParameters detection_parameters(DeClickParameters parameters) {
    parameters.enabled = true;
    parameters.repair_percent = 100;
    return parameters;
}
} // namespace

ClickCandidateAnalyzer::ClickCandidateAnalyzer(
    DeClickParameters parameters,
    std::size_t channels,
    std::uint64_t first_frame,
    std::uint64_t last_frame
) :
    processor_(detection_parameters(parameters), 48000, channels), channels_(channels),
    work_(kBlockFrames * channels), history_((processor_.latency_frames() + 1) * channels),
    first_frame_(first_frame), last_frame_(last_frame) {
    result_.candidates.reserve(kMaximumCandidates);
}

void ClickCandidateAnalyzer::finish_candidate() {
    if (!current_)
        return;
    ++result_.total_candidates;
    if (result_.candidates.size() < kMaximumCandidates)
        result_.candidates.push_back(*current_);
    current_.reset();
}

void ClickCandidateAnalyzer::process_interleaved(const float* samples, std::size_t frames) {
    if (!samples && frames)
        throw std::invalid_argument("missing click analysis samples");
    const auto latency = processor_.latency_frames();
    const auto capacity = latency + 1;
    for (std::size_t offset = 0; offset < frames;) {
        const auto count = std::min(kBlockFrames, frames - offset);
        std::copy_n(samples + offset * channels_, count * channels_, work_.data());
        processor_.process_interleaved(work_.data(), count, channels_);
        for (std::size_t i = 0; i < count; ++i, ++input_frames_) {
            const auto slot = static_cast<std::size_t>(input_frames_ % capacity);
            for (std::size_t channel = 0; channel < channels_; ++channel) {
                const float sample = samples[(offset + i) * channels_ + channel];
                if (!std::isfinite(sample))
                    throw std::runtime_error("non-finite source sample");
                history_[slot * channels_ + channel] = sample;
            }
            if (input_frames_ < latency)
                continue;
            const auto frame = input_frames_ - latency;
            if (frame < first_frame_ || frame >= last_frame_)
                continue;
            const auto delayed = static_cast<std::size_t>(frame % capacity);
            std::uint32_t channel_mask = 0;
            float maximum = 0;
            for (std::size_t channel = 0; channel < channels_; ++channel) {
                const auto difference = std::abs(
                    work_[i * channels_ + channel] - history_[delayed * channels_ + channel]
                );
                if (difference > kDifferenceFloor) {
                    channel_mask |= std::uint32_t{1} << channel;
                    maximum = std::max(maximum, difference);
                }
            }
            if (current_ && frame > current_->end_frame + kFramesPerMilli)
                finish_candidate();
            if (channel_mask) {
                if (!current_)
                    current_ = ClickCandidate{.start_frame = frame};
                current_->end_frame = frame + 1;
                current_->channel_mask |= channel_mask;
                current_->maximum_difference = std::max(current_->maximum_difference, maximum);
            }
        }
        offset += count;
    }
}

ClickAnalysisResult ClickCandidateAnalyzer::result() const {
    auto result = result_;
    result.analyzed_frames = input_frames_;
    if (current_) {
        ++result.total_candidates;
        if (result.candidates.size() < kMaximumCandidates)
            result.candidates.push_back(*current_);
    }
    return result;
}

ClickAnalysisResult analyze_clicks(
    const std::string& path,
    std::uint64_t start_millis,
    std::uint64_t end_millis,
    DeClickParameters parameters,
    std::stop_token cancellation
) {
    if (start_millis >= end_millis || end_millis - start_millis > 300000 || end_millis > 14400000)
        throw std::invalid_argument("click analysis requires up to five minutes of original audio");
    if (cancellation.stop_requested())
        throw std::runtime_error("click analysis cancelled");
    const auto modified = std::filesystem::last_write_time(path);
    const auto size = std::filesystem::file_size(path);
    const auto source = probe(path);
    if (!source.has_audio || end_millis > source.duration_millis)
        throw std::invalid_argument("click analysis outside source");
    const auto decode_start = start_millis > 10 ? start_millis - 10 : 0;
    const auto decode_end = std::min(end_millis + 10, source.duration_millis);
    PlaybackAdjustment adjustment;
    adjustment.trim_start_millis = decode_start;
    adjustment.trim_end_millis = decode_end;
    PlaybackSession session(
        path,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    const auto expected = session.output_frame_count();
    // A real trajectory on both sides is required, not decoder zero padding.
    const auto first_frame = std::max(
        (start_millis - decode_start) * kFramesPerMilli,
        decode_start == 0 ? std::uint64_t{2} : 0
    );
    const auto last_frame = std::min(
        (end_millis - decode_start) * kFramesPerMilli,
        expected - std::min<std::uint64_t>(expected, 100)
    );
    ClickCandidateAnalyzer analyzer(parameters, session.channel_count(), first_frame, last_frame);
    std::vector<float> buffer(kBlockFrames * session.channel_count());
    std::uint64_t frames = 0;
    while (!cancellation.stop_requested() && frames < expected) {
        const auto count = session.read(
            buffer.data(),
            static_cast<std::size_t>(std::min<std::uint64_t>(kBlockFrames, expected - frames))
        );
        if (count) {
            analyzer.process_interleaved(buffer.data(), count);
            frames += count;
        } else if (session.is_ended() || session.is_stopped())
            break;
        else
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    if (cancellation.stop_requested())
        throw std::runtime_error("click analysis cancelled");
    if (frames != expected || modified != std::filesystem::last_write_time(path)
        || size != std::filesystem::file_size(path))
        throw std::runtime_error("click analysis source changed or decode was incomplete");
    auto result = analyzer.result();
    const auto offset = decode_start * kFramesPerMilli;
    for (auto& candidate : result.candidates) {
        candidate.start_frame += offset;
        candidate.end_frame += offset;
    }
    return result;
}
} // namespace echo::audio
