#include "click_analysis_controller.hpp"
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QThread>
#include <atomic>
#include <cassert>
#include <stdexcept>

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    std::atomic<int> active = 0, maximum = 0, calls = 0;
    const auto wait = [&](auto predicate) {
        QElapsedTimer timer;
        timer.start();
        while (!predicate() && timer.elapsed() < 5000) {
            QCoreApplication::processEvents();
            QThread::msleep(1);
        }
        assert(predicate());
    };
    ClickAnalysisController controller(
        nullptr,
        [&](const std::string& path,
            std::uint64_t start,
            std::uint64_t end,
            echo::audio::DeClickParameters parameters,
            std::stop_token token) {
            ++calls;
            maximum.store(std::max(maximum.load(), ++active));
            assert(start == 1000 && end == 2000 && parameters.sensitivity_percent == 85);
            if (path == "old")
                while (!token.stop_requested())
                    QThread::msleep(1);
            --active;
            if (path == "failure")
                throw std::runtime_error("decode failed");
            return echo::audio::ClickAnalysisResult{
                .candidates =
                    {{.start_frame = 48000,
                      .end_frame = 48001,
                      .channel_mask = 2,
                      .maximum_difference = 0.5F}},
                .total_candidates = 1,
                .analyzed_frames = 48000
            };
        }
    );
    controller.scan("old", 1000, 2000, 85, 1000, "old-key");
    wait([&] { return active == 1; });
    controller.scan("superseded", 1000, 2000, 85, 1000, "superseded-key");
    controller.scan("latest", 1000, 2000, 85, 1000, "latest-key");
    wait([&] { return !controller.running(); });
    assert(maximum == 1 && calls == 2 && controller.resultKey() == "latest-key");
    const auto item = controller.result().value("items").toList().first().toMap();
    assert(
        item.value("startMillis").toLongLong() == 1000
        && item.value("endMillis").toLongLong() == 1001 && item.value("channelMask").toUInt() == 2
    );
    controller.scan("failure", 1000, 2000, 85, 1000, "fail");
    wait([&] { return !controller.running(); });
    assert(!controller.errorText().isEmpty() && controller.result().isEmpty());
    controller.scan("old", 1000, 2000, 85, 1000, "cancel");
    wait([&] { return active == 1; });
    controller.cancel();
    wait([&] { return !controller.running(); });
    assert(
        controller.result().isEmpty() && controller.errorText().isEmpty()
        && !controller.cancelling()
    );
    controller.scan("x", 0, 300001, 85, 1000, "invalid");
    assert(!controller.running() && !controller.errorText().isEmpty());
    controller.scan("recovered", 1000, 2000, 85, 1000, "recovered");
    wait([&] { return !controller.running(); });
    assert(controller.errorText().isEmpty() && controller.resultKey() == "recovered");
}
