#pragma once
#include "echo/audio/source_edit_plan.hpp"
#include <algorithm>
#include <atomic>
#include <cstring>
#include <vector>

namespace echo::audio {
/// Fixed-capacity single-producer/single-consumer frame ring.
///
/// Indices are monotonically increasing (wrapping the counter space is
/// impossible at 48 kHz), so "empty" means `write == read` and "full" means
/// `write - read == capacity`; the array offset is `index % capacity`.
/// Only the producer writes `write_index`; only the consumer writes
/// `read_index`.
class PlaybackFrameRing {
  public:
    PlaybackFrameRing(std::size_t capacity_frames, std::size_t channels) :
        data_(capacity_frames * channels, 0.0F), capacity_frames_(capacity_frames),
        channels_(channels), source_frames_(capacity_frames, kNoSourceFrame) {}

    std::size_t write(const float* frames, const std::uint64_t* source_frames, std::size_t count) {
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
            if (source_frames != nullptr) {
                std::copy_n(source_frames + written, contiguous, source_frames_.data() + first);
                if (chunk > contiguous) {
                    std::copy_n(
                        source_frames + written + contiguous,
                        chunk - contiguous,
                        source_frames_.data()
                    );
                }
            } else {
                std::fill_n(source_frames_.data() + first, contiguous, kNoSourceFrame);
                if (chunk > contiguous) {
                    std::fill_n(source_frames_.data(), chunk - contiguous, kNoSourceFrame);
                }
            }
            write_index_.store(write_index + chunk, std::memory_order_release);
            written += chunk;
        }
        return written;
    }

    std::size_t read(float* output, std::size_t count, std::uint64_t* last_source_frame) {
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
            if (last_source_frame != nullptr) {
                for (std::size_t index = 0; index < chunk; ++index) {
                    const std::size_t source_index = (first + index) % capacity_frames_;
                    if (source_frames_[source_index] != kNoSourceFrame) {
                        *last_source_frame = source_frames_[source_index];
                    }
                }
            }
            read_index_.store(read_index + chunk, std::memory_order_release);
            read_frames += chunk;
        }
        return read_frames;
    }

    /// Discards all buffered frames after the session's read-epoch barrier has
    /// excluded in-flight consumers.
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
    std::vector<std::uint64_t> source_frames_;
    std::atomic<std::size_t> read_index_{0};
    std::atomic<std::size_t> write_index_{0};
};

} // namespace echo::audio
