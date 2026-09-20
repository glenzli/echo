#include "selection_transcription_controller.hpp"
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QSemaphore>
#include <QThread>
#include <QThreadPool>
#include <atomic>
#include <cassert>
#include <stdexcept>

template <class F> void waitFor(F ready) {
    QElapsedTimer timer;
    timer.start();
    while (!ready()) {
        assert(timer.elapsed() < 5000);
        QCoreApplication::processEvents();
        QThread::msleep(1);
    }
}
int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QSemaphore release;
    std::atomic<int> calls = 0;
    SelectionTranscriptionController controller(
        "catalog",
        "cache",
        nullptr,
        [&](const QString&, const QString&, const QString& id, qint64, qint64, const QString&) {
            ++calls;
            if (id == "slow")
                release.acquire();
            if (id == "failure")
                throw std::runtime_error("private error");
            return id;
        }
    );
    controller.request("slow", 0, 1000, "", "old revision");
    waitFor([&] { return calls == 1; });
    controller.discard();
    controller.request("new", 0, 1000, "", "new revision");
    assert(controller.running() && controller.discarded() && calls == 1);
    release.release();
    waitFor([&] { return !controller.running(); });
    assert(controller.resultJson().isEmpty() && controller.requestKey().isEmpty());
    controller.request("fresh", 0, 1000, "", "fresh revision");
    waitFor([&] { return !controller.running(); });
    assert(controller.resultJson() == "fresh" && controller.requestKey() == "fresh revision");
    controller.request("failure", 0, 1000, "", "error");
    waitFor([&] { return !controller.running(); });
    assert(!controller.errorText().isEmpty() && !controller.errorText().contains("private"));
    controller.request("too long", 0, 300001, "", "invalid");
    assert(!controller.running() && calls == 3);
    QThreadPool::globalInstance()->waitForDone();
}
