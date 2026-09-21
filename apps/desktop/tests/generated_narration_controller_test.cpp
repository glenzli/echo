#include "echo-desktop-bridge/src/lib.rs.h"
#include "generated_narration_controller.hpp"
#include <QAudioOutput>
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QGuiApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMediaPlayer>
#include <QSemaphore>
#include <QThread>
#include <QThreadPool>
#include <atomic>
#include <cassert>
#include <cstdio>
#include <stdexcept>

template <class F> void waitFor(F ready, qint64 timeout = 5000) {
    QElapsedTimer timer;
    timer.start();
    while (!ready()) {
        assert(timer.elapsed() < timeout);
        QCoreApplication::processEvents();
        QThread::msleep(1);
    }
}
class Result final : public NarrationResult {
    QString name_;

  public:
    explicit Result(QString name) : name_(std::move(name)) {}
    mutable int attempts = 0;
    QString details() const override {
        return QString("{\"duration_millis\":1000,\"input_text\":\"%1\"}").arg(name_);
    }
    QString accept(const QString&, const QString&, bool) const override {
        if (++attempts == 1)
            throw std::runtime_error("private error");
        return name_;
    }
};
int main(int argc, char** argv) {
    const auto liveRoot = qEnvironmentVariable("ECHO_NARRATION_LIVE_ROOT");
    std::unique_ptr<QCoreApplication> app;
    if (liveRoot.isEmpty())
        app = std::make_unique<QCoreApplication>(argc, argv);
    else
        app = std::make_unique<QGuiApplication>(argc, argv);
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
        return std::make_shared<Result>(text);
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
        assert(id == "fresh");
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

    GeneratedNarrationController bank("catalog", nullptr, generate);
    bank.request("one", "");
    waitFor([&] { return !bank.running(); });
    const auto first = bank.selectedCandidateId();
    const auto firstPath = bank.audioUrl().toLocalFile();
    bank.request("two", "");
    waitFor([&] { return !bank.running(); });
    const auto second = bank.selectedCandidateId();
    const auto secondPath = bank.audioUrl().toLocalFile();
    bank.request("three", "");
    waitFor([&] { return !bank.running(); });
    assert(bank.candidates().size() == 3 && QFile::exists(firstPath) && QFile::exists(secondPath));
    const int fullCalls = calls;
    bank.request("four", "");
    assert(!bank.running() && calls == fullCalls && !bank.errorText().isEmpty());
    bank.selectCandidate(first);
    assert(bank.audioUrl().toLocalFile() == firstPath && bank.detailsJson().contains("one"));
    bank.removeSelected();
    assert(bank.candidates().size() == 2 && !QFile::exists(firstPath) && QFile::exists(secondPath));
    bank.selectCandidate(second);
    bank.request("failure", "");
    waitFor([&] { return !bank.running(); });
    assert(
        bank.candidates().size() == 2 && bank.selectedCandidateId() == second
        && !bank.errorText().isEmpty()
    );
    QString selectedAcceptance;
    QObject::connect(&bank, &GeneratedNarrationController::accepted, [&](const QString& id) {
        selectedAcceptance = id;
    });
    bank.accept("project", false);
    waitFor([&] { return !bank.accepting(); });
    assert(bank.candidates().size() == 2 && selectedAcceptance.isEmpty());
    bank.accept("project", false);
    waitFor([&] { return !bank.accepting(); });
    assert(selectedAcceptance == "two" && bank.candidates().isEmpty());
    waitFor([&] { return !QFile::exists(secondPath); });

    // Explicit opt-in integration: real local SDK jobs, muted audio-device audition,
    // and admission into a caller-owned catalog. No production library is written.
    if (!liveRoot.isEmpty()) {
        fprintf(stderr, "Live integration: unit lifecycle passed\n");
        assert(QDir().mkpath(liveRoot));
        const auto catalog = liveRoot + "/catalog.sqlite";
        auto session =
            echo::desktop::open_session(catalog.toStdString(), (liveRoot + "/cache").toStdString());
        assert(session->session_list_assets().empty());
        GeneratedNarrationController live(catalog);
        const QString textA = QStringLiteral("这是一段用于测试候选对比的合成旁白。");
        const QString textB = QStringLiteral("这是第二个候选，用来验证选择和试听流程。");
        const QString endpoint = QStringLiteral("http://127.0.0.1:8787");
        live.request(textA, endpoint);
        fprintf(stderr, "Live integration: generating candidate A\n");
        waitFor([&] { return !live.running(); }, 240000);
        assert(live.errorText().isEmpty() && live.candidates().size() == 1);
        const auto idA = live.selectedCandidateId();
        const auto receiptA = QJsonDocument::fromJson(live.detailsJson().toUtf8()).object();
        fprintf(stderr, "Live integration: candidate A ready\n");
        live.request(textB, endpoint);
        waitFor([&] { return !live.running(); }, 240000);
        assert(live.errorText().isEmpty() && live.candidates().size() == 2);
        const auto idB = live.selectedCandidateId();
        fprintf(stderr, "Live integration: candidate B ready\n");
        QAudioOutput audio;
        audio.setVolume(0);
        QMediaPlayer media;
        media.setAudioOutput(&audio);
        QObject::connect(&media, &QMediaPlayer::errorOccurred, [&](QMediaPlayer::Error error) {
            fprintf(stderr, "Live integration: playback error %d\n", static_cast<int>(error));
        });
        for (const auto& id : {idB, idA}) {
            live.selectCandidate(id);
            media.setSource(live.audioUrl());
            media.play();
            fprintf(stderr, "Live integration: audition started\n");
            waitFor([&] { return media.position() > 0; }, 15000);
            media.stop();
            media.setSource({});
        }
        assert(session->session_list_assets().empty());
        QString admitted;
        QObject::connect(&live, &GeneratedNarrationController::accepted, [&](const QString& id) {
            admitted = id;
        });
        live.accept("", true);
        fprintf(stderr, "Live integration: accepting selected candidate\n");
        waitFor([&] { return !live.accepting(); }, 15000);
        assert(!admitted.isEmpty() && live.candidates().isEmpty());
        const auto assets = session->session_list_assets();
        assert(assets.size() == 1);
        const auto disclosure =
            QString::fromStdString(std::string(assets[0].source_disclosure_json));
        assert(disclosure.contains(textA) && !disclosure.contains(textB));
        QFile report(liveRoot + "/validation.json");
        assert(report.open(QIODevice::WriteOnly));
        report.write(QJsonDocument(
                         QJsonObject{
                             {"ok", true},
                             {"candidatesGenerated", 2},
                             {"candidatesAuditioned", 2},
                             {"acceptedFirst", true},
                             {"unacceptedExcluded", true},
                             {"acceptedOutputHash", receiptA.value("output_hash")}
                         }
        ).toJson());
    }
}
