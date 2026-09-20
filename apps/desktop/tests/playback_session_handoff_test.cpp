#include "playback_session_handoff.hpp"

#include <QCoreApplication>
#include <QFile>
#include <QTemporaryDir>
#include <array>
#include <atomic>
#include <cassert>
#include <cstring>
#include <iostream>
#include <semaphore>
#include <thread>

namespace {
QString makeSource(const QTemporaryDir& root) {
    QByteArray bytes(44 + 4800 * 4, '\0');
    auto put = [&](int offset, auto value) {
        std::memcpy(bytes.data() + offset, &value, sizeof(value));
    };
    std::memcpy(bytes.data(), "RIFF", 4);
    put(4, quint32(bytes.size() - 8));
    std::memcpy(bytes.data() + 8, "WAVEfmt ", 8);
    put(16, quint32(16));
    put(20, quint16(1));
    put(22, quint16(2));
    put(24, quint32(48000));
    put(28, quint32(192000));
    put(32, quint16(4));
    put(34, quint16(16));
    std::memcpy(bytes.data() + 36, "data", 4);
    put(40, quint32(bytes.size() - 44));
    for (int offset = 44; offset < bytes.size(); offset += 2)
        put(offset, qint16(5000));
    const QString path = root.filePath("handoff.wav");
    QFile file(path);
    assert(file.open(QIODevice::WriteOnly));
    assert(file.write(bytes) == bytes.size());
    return path;
}

struct Sessions {
    const std::thread::id control = std::this_thread::get_id();
    int destroyed = 0;
    std::shared_ptr<echo::audio::PlaybackSession> create(const QString& path) {
        return {new echo::audio::PlaybackSession(path.toStdString()), [this](auto* session) {
                    assert(std::this_thread::get_id() == control);
                    ++destroyed;
                    delete session;
                }};
    }
};

void inFlightReadSurvivesReplaceAndStop(const QString& path) {
    Sessions sessions;
    PlaybackSessionHandoff handoff;
    auto first = sessions.create(path);
    const std::weak_ptr firstWeak = first;
    assert(handoff.publish(first));
    std::binary_semaphore entered(0), finish(0);
    std::thread callback([&] {
        const auto read = handoff.read();
        auto* const borrowed = read.session();
        assert(borrowed != nullptr);
        entered.release();
        finish.acquire();
        // Access after replacement and stop would fail without deferred ownership.
        assert(borrowed->duration_millis() == 100);
        std::array<float, 128> buffer{};
        assert(borrowed->read(buffer.data(), 64) == 0);
    });
    entered.acquire();
    first->stop();
    first.reset();
    auto second = sessions.create(path);
    const std::weak_ptr secondWeak = second;
    assert(!handoff.publish(second));
    assert(!firstWeak.expired());
    second->stop();
    second.reset();
    assert(!handoff.publish(nullptr));
    assert(!handoff.collectRetired());
    finish.release();
    callback.join();
    // Ending the callback must not run the session deleter on that thread.
    assert(sessions.destroyed == 0);
    assert(handoff.collectRetired());
    assert(firstWeak.expired() && secondWeak.expired());
    assert(sessions.destroyed == 2);
    assert(handoff.read().session() == nullptr);
}

void repeatedReplacementReclaimsEverySession(const QString& path) {
    Sessions sessions;
    PlaybackSessionHandoff handoff;
    std::atomic<bool> done{false};
    std::atomic<int> reads{0};
    std::binary_semaphore entered(0);
    auto current = sessions.create(path);
    assert(handoff.publish(current));
    std::thread callback([&] {
        std::array<float, 256> buffer{};
        bool first = true;
        while (!done.load()) {
            {
                const auto read = handoff.read();
                if (const auto* session = read.session()) {
                    assert(session->sample_rate() == 48000 && session->channel_count() == 2);
                    assert(read.session()->read(buffer.data(), 128) <= 128);
                    reads.fetch_add(1);
                }
                if (first) {
                    entered.release();
                    first = false;
                }
            }
            std::this_thread::yield();
        }
    });
    entered.acquire();
    std::vector<std::weak_ptr<echo::audio::PlaybackSession>> previous;
    constexpr int replacements = 200;
    for (int i = 0; i < replacements; ++i) {
        previous.push_back(current);
        current->stop();
        auto next = sessions.create(path);
        (void)handoff.publish(next);
        current = std::move(next);
        (void)handoff.collectRetired();
    }
    current->stop();
    previous.push_back(current);
    (void)handoff.publish(nullptr);
    current.reset();
    done.store(true);
    callback.join();
    assert(handoff.collectRetired());
    for (const auto& weak : previous)
        assert(weak.expired());
    assert(sessions.destroyed == replacements + 1);
    assert(reads.load() > 0);
    std::cout << replacements << " replacements; " << sessions.destroyed
              << " sessions reclaimed on control thread; " << reads << " callback reads\n";
}
} // namespace

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir root;
    assert(root.isValid());
    const QString path = makeSource(root);
    inFlightReadSurvivesReplaceAndStop(path);
    repeatedReplacementReclaimsEverySession(path);
}
