//! Main-thread ownership of sessions borrowed by the device callback.
#pragma once

#include "echo/audio/playback_stream.hpp"

#include <atomic>
#include <cstdint>
#include <memory>
#include <utility>
#include <vector>

// publish/collectRetired belong to one control thread. Borrowed reads perform
// no allocation, reference counting, destruction or locking on the callback.
// The device must stop invoking callbacks before this owner is destroyed.
class PlaybackSessionHandoff {
  public:
    class Read {
      public:
        explicit Read(PlaybackSessionHandoff& owner) : owner_(owner) {
            owner_.readers_.fetch_add(1, std::memory_order_seq_cst);
            session_ = owner_.callback_.load(std::memory_order_seq_cst);
        }
        ~Read() {
            owner_.readers_.fetch_sub(1, std::memory_order_seq_cst);
        }
        Read(const Read&) = delete;
        Read& operator=(const Read&) = delete;
        [[nodiscard]] echo::audio::PlaybackStream* session() const {
            return session_;
        }

      private:
        PlaybackSessionHandoff& owner_;
        echo::audio::PlaybackStream* session_;
    };

    [[nodiscard]] Read read() {
        return Read(*this);
    }

    // True means no retired session remains; otherwise collect after a later
    // callback completes, including when playback has been stopped or paused.
    [[nodiscard]] bool publish(std::shared_ptr<echo::audio::PlaybackStream> session) {
        if (current_ != session) {
            if (current_)
                retired_.push_back(std::move(current_));
            current_ = std::move(session);
            callback_.store(current_.get(), std::memory_order_seq_cst);
        }
        return collectRetired();
    }

    [[nodiscard]] bool collectRetired() {
        // In the sequentially consistent order, a reader of the old pointer
        // increments before publish's store and this load. A zero count means
        // that reader has finished; any later reader sees the new pointer.
        if (readers_.load(std::memory_order_seq_cst) == 0)
            retired_.clear();
        return retired_.empty();
    }

  private:
    static_assert(std::atomic<std::uint32_t>::is_always_lock_free);
    static_assert(std::atomic<echo::audio::PlaybackStream*>::is_always_lock_free);
    std::atomic<std::uint32_t> readers_{0};
    std::atomic<echo::audio::PlaybackStream*> callback_{nullptr};
    std::shared_ptr<echo::audio::PlaybackStream> current_;
    std::vector<std::shared_ptr<echo::audio::PlaybackStream>> retired_;
};
