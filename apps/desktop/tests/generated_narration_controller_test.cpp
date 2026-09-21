#include "generated_narration_controller.hpp"
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
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
class Result final : public NarrationResult {
  public:
    mutable int attempts = 0;
    QString details() const override {
        return "{\"duration_millis\":1000}";
    }
    QString accept(const QString&, const QString&, bool) const override {
        if (++attempts == 1)
            throw std::runtime_error("private error");
        return "accepted-id";
    }
};
int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QSemaphore release;
    std::atomic<int> calls = 0;
    QString directory;
    auto generate = [&](const QString& text,
                        const QString& root,
                        const QString&) -> std::shared_ptr<NarrationResult> {
        directory = root;
        ++calls;
        if (text == "slow")
            release.acquire();
        if (text == "failure")
            throw std::runtime_error("private detail");
        QFile file(root + "/narration.wav");
        assert(file.open(QIODevice::WriteOnly));
        file.write("fixture");
        return std::make_shared<Result>();
    };
    GeneratedNarrationController controller("catalog", nullptr, generate);
    controller.request("slow", "");
    waitFor([&] { return calls == 1; });
    controller.discard();
    controller.request("overlap", "");
    assert(calls == 1 && controller.running());
    release.release();
    waitFor([&] { return !controller.running(); });
    assert(controller.detailsJson().isEmpty() && controller.audioUrl().isEmpty());
    waitFor([&] { return !QDir(directory).exists(); });
    controller.request("fresh", "");
    waitFor([&] { return !controller.running(); });
    assert(
        !controller.detailsJson().isEmpty() && QFile::exists(controller.audioUrl().toLocalFile())
    );
    controller.accept("", true);
    waitFor([&] { return !controller.accepting(); });
    assert(!controller.errorText().isEmpty() && !controller.errorText().contains("private"));
    assert(!controller.detailsJson().isEmpty());
    int accepted = 0;
    QObject::connect(&controller, &GeneratedNarrationController::accepted, [&](const QString& id) {
        assert(id == "accepted-id");
        ++accepted;
    });
    controller.accept("", true);
    controller.accept("", true);
    controller.discard();
    waitFor([&] { return !controller.accepting(); });
    assert(accepted == 1 && controller.detailsJson().isEmpty());
    waitFor([&] { return !QDir(directory).exists(); });
    controller.request("failure", "");
    waitFor([&] { return !controller.running(); });
    assert(!controller.errorText().isEmpty() && !controller.errorText().contains("private"));
    controller.request(QString(501, 'x'), "");
    assert(!controller.running() && calls == 3);
    auto* destroyed = new GeneratedNarrationController("catalog", nullptr, generate);
    destroyed->request("slow", "");
    waitFor([&] { return calls == 4; });
    delete destroyed;
    release.release();
    QThreadPool::globalInstance()->waitForDone();
    assert(!QDir(directory).exists());
}
