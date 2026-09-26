#include "echo-desktop-bridge/src/lib.rs.h"
#include "generated_audio_controller.hpp"
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
class Result final : public GeneratedAudioResult {
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
    const auto soundRoot = qEnvironmentVariable("ECHO_SOUND_EFFECT_LIVE_ROOT");
    const bool soundLive = !soundRoot.isEmpty();
    const auto liveRoot = soundLive ? soundRoot : qEnvironmentVariable("ECHO_NARRATION_LIVE_ROOT");
    std::unique_ptr<QCoreApplication> app;
    if (liveRoot.isEmpty())
        app = std::make_unique<QCoreApplication>(argc, argv);
    else
        app = std::make_unique<QGuiApplication>(argc, argv);
    QSemaphore release;
    std::atomic<int> calls = 0;
    QString directory;
    auto generate = [&](const GeneratedAudioRequest& input,
                        const QString& root,
                        const QString&) -> std::shared_ptr<GeneratedAudioResult> {
        const auto& text = input.text;
        directory = root;
        ++calls;
        if (text == "slow")
            release.acquire();
        if (text == "failure")
            throw std::runtime_error("private detail");
        QFile file(root + (input.durationSeconds > 0 ? "/sound.wav" : "/narration.wav"));
        assert(file.open(QIODevice::WriteOnly));
        file.write("fixture");
        return std::make_shared<Result>(text);
    };
    GeneratedAudioController controller("catalog", nullptr, generate);
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
    QObject::connect(&controller, &GeneratedAudioController::accepted, [&](const QString& id) {
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
    auto* destroyed = new GeneratedAudioController("catalog", nullptr, generate);
    destroyed->request("slow", "");
    waitFor([&] { return calls == 4; });
    delete destroyed;
    release.release();
    QThreadPool::globalInstance()->waitForDone();
    assert(!QDir(directory).exists());

    GeneratedAudioController bank("catalog", nullptr, generate);
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
    QObject::connect(&bank, &GeneratedAudioController::accepted, [&](const QString& id) {
        selectedAcceptance = id;
    });
    bank.accept("project", false);
    waitFor([&] { return !bank.accepting(); });
    assert(bank.candidates().size() == 2 && selectedAcceptance.isEmpty());
    bank.accept("project", false);
    waitFor([&] { return !bank.accepting(); });
    assert(selectedAcceptance == "two" && bank.candidates().isEmpty());
    waitFor([&] { return !QFile::exists(secondPath); });

    GeneratedAudioRequest captured;
    GeneratedAudioController sound(
        "catalog",
        nullptr,
        [&](const GeneratedAudioRequest& input, const QString& root, const QString& endpoint) {
            captured = input;
            return generate(input, root, endpoint);
        },
        GeneratedAudioController::Kind::SoundMaterial,
        [](const QString&, const QString&) { return QStringLiteral("prepared fixture"); }
    );
    const int beforeSound = calls;
    sound.request("wrong intent", "");
    sound.requestSoundMaterial("rain", 0, true, "");
    sound.requestSoundMaterial("rain", 31, true, "");
    assert(!sound.running() && calls == beforeSound);
    sound.requestSoundMaterial("  rain  ", 8, true, "");
    waitFor([&] { return !sound.running(); });
    assert(captured.text == "rain" && captured.durationSeconds == 8 && captured.ambience);
    assert(sound.audioUrl().toLocalFile().endsWith("/sound.wav"));
    assert(QFile::exists(sound.audioUrl().toLocalFile()));
    sound.discard();
    sound.requestSoundMaterial("piano", 8, false, "", "stable_audio_3_small_music");
    waitFor([&] { return !sound.running(); });
    assert(captured.model == QStringLiteral("stable_audio_3_small_music"));
    sound.discard();
    const int beforeInvalid = calls;
    sound.requestSoundMaterial("door", 12, false, "", "stable_audio_open_small");
    sound.requestSoundMaterial("piano", 8, false, "", "unknown");
    assert(!sound.running() && calls == beforeInvalid);
    sound.requestSoundMaterial("slow", 8, false, "");
    waitFor([&] { return calls == beforeInvalid + 1; });
    sound.discard();
    release.release();
    waitFor([&] { return !sound.running(); });
    assert(sound.candidates().isEmpty());
    controller.requestSoundMaterial("wrong intent", 8, true, "");
    assert(!controller.running());

    // Preparation is retained for seed/duration/model retries; stopping before
    // handoff must never submit audio, and failure must preserve old candidates.
    std::atomic<int> preparations = 0, soundCalls = 0;
    QSemaphore prepareRelease, soundRelease;
    bool rejectPreparation = false, rejectSound = false;
    GeneratedAudioController staged(
        "catalog",
        nullptr,
        [&](const GeneratedAudioRequest& input, const QString& root, const QString& endpoint) {
            ++soundCalls;
            assert(input.preparation == "ready:" + input.text);
            if (input.text == "slow sound")
                soundRelease.acquire();
            if (rejectSound)
                throw std::runtime_error("private audio failure");
            return generate(input, root, endpoint);
        },
        GeneratedAudioController::Kind::SoundMaterial,
        [&](const QString& text, const QString&) {
            ++preparations;
            if (text == "slow prepare")
                prepareRelease.acquire();
            if (rejectPreparation)
                throw std::runtime_error("private preparation failure");
            return "ready:" + text;
        }
    );
    staged.requestSoundMaterial("rain", 3, true, "one");
    waitFor([&] { return !staged.running(); });
    const auto retained = staged.selectedCandidateId();
    assert(preparations == 1 && soundCalls == 1);
    rejectSound = true;
    staged.requestSoundMaterial("rain", 4, false, "one", "stable_audio_3_small_music");
    waitFor([&] { return !staged.running(); });
    assert(preparations == 1 && soundCalls == 2 && staged.selectedCandidateId() == retained);
    assert(!staged.errorText().isEmpty() && !staged.errorText().contains("private"));
    rejectSound = false;
    staged.requestSoundMaterial("rain", 3, true, "one");
    waitFor([&] { return !staged.running(); });
    assert(preparations == 1 && soundCalls == 3 && staged.candidates().size() == 2);
    staged.removeSelected();
    rejectPreparation = true;
    staged.requestSoundMaterial("new prompt", 3, true, "one");
    waitFor([&] { return !staged.running(); });
    assert(preparations == 2 && soundCalls == 3 && staged.selectedCandidateId() == retained);
    assert(
        staged.errorText().contains("No audio was generated")
        && !staged.errorText().contains("private")
    );
    rejectPreparation = false;
    staged.requestSoundMaterial("slow prepare", 3, true, "one");
    waitFor([&] { return preparations == 3; });
    assert(staged.preparing());
    staged.stop();
    assert(staged.stopping() && staged.running());
    prepareRelease.release();
    waitFor([&] { return !staged.running(); });
    assert(soundCalls == 3 && staged.candidates().size() == 1 && !staged.stopping());
    staged.requestSoundMaterial("slow sound", 3, true, "one");
    waitFor([&] { return soundCalls == 4; });
    assert(!staged.preparing());
    staged.stop();
    soundRelease.release();
    waitFor([&] { return !staged.running(); });
    assert(staged.candidates().size() == 1 && staged.selectedCandidateId() == retained);
    staged.requestSoundMaterial("rain", 3, true, "two");
    waitFor([&] { return !staged.running(); });
    assert(preparations == 5); // Endpoint changes invalidate reuse.
    staged.discard();
    staged.requestSoundMaterial("rain", 3, true, "two");
    waitFor([&] { return !staged.running(); });
    assert(preparations == 6); // A new dialog batch starts clean.

    // Explicit opt-in integration: real local SDK jobs, muted audio-device audition,
    // and admission into a caller-owned catalog. No production library is written.
    if (!liveRoot.isEmpty()) {
        fprintf(stderr, "Live integration: unit lifecycle passed\n");
        assert(QDir().mkpath(liveRoot));
        const auto catalog = liveRoot + "/catalog.sqlite";
        auto session = soundLive ? echo::desktop::open_editor_session(liveRoot.toStdString())
                                 : echo::desktop::open_session(
                                       catalog.toStdString(),
                                       (liveRoot + "/cache").toStdString()
                                   );
        assert(session->session_list_assets().empty());
        GeneratedAudioController live(
            catalog,
            nullptr,
            {},
            soundLive ? GeneratedAudioController::Kind::SoundMaterial
                      : GeneratedAudioController::Kind::Narration
        );
        const auto model =
            qEnvironmentVariable("ECHO_SOUND_MATERIAL_MODEL", "stable_audio_3_small_sfx");
        const bool musicLive = model == QStringLiteral("stable_audio_3_small_music");
        const QString defaultTextA =
            musicLive ? QStringLiteral(
                            "Soft sparse piano notes, warm gentle instrumental background, no "
                            "vocals or drums."
                        )
            : soundLive
                ? QStringLiteral(
                      "Gentle rain outside a window, soft steady patter, no speech or music."
                  )
                : QStringLiteral("这是一段用于测试候选对比的合成旁白。");
        const QString defaultTextB =
            musicLive   ? QStringLiteral(
                              "Warm slowly evolving ambient synthesizer pad, calm "
                              "instrumental texture, no vocals or drums."
                          )
            : soundLive ? QStringLiteral(
                              "A wooden door closing with a soft creak and a single "
                              "latch click, no speech or music."
                          )
                        : QStringLiteral("这是第二个候选，用来验证选择和试听流程。");
        const QString textA = qEnvironmentVariable("ECHO_GENERATION_PROMPT_A", defaultTextA);
        const QString textB = qEnvironmentVariable("ECHO_GENERATION_PROMPT_B", defaultTextB);
        const int seconds = qEnvironmentVariableIntValue("ECHO_GENERATION_SECONDS") > 0
                                ? qEnvironmentVariableIntValue("ECHO_GENERATION_SECONDS")
                                : 8;
        const QString endpoint = QStringLiteral("http://127.0.0.1:8787");
        if (soundLive)
            live.requestSoundMaterial(textA, seconds, !musicLive, endpoint, model);
        else
            live.request(textA, endpoint);
        fprintf(stderr, "Live integration: generating candidate A\n");
        waitFor([&] { return !live.running(); }, 240000);
        assert(live.errorText().isEmpty() && live.candidates().size() == 1);
        const auto idA = live.selectedCandidateId();
        const auto receiptA = QJsonDocument::fromJson(live.detailsJson().toUtf8()).object();
        QFile candidateAudio(live.audioUrl().toLocalFile());
        assert(candidateAudio.open(QIODevice::ReadOnly));
        const auto originalBytes = candidateAudio.readAll();
        candidateAudio.close();
        fprintf(stderr, "Live integration: candidate A ready\n");
        if (soundLive)
            live.requestSoundMaterial(textB, seconds, false, endpoint, model);
        else
            live.request(textB, endpoint);
        waitFor([&] { return !live.running(); }, 240000);
        assert(live.errorText().isEmpty() && live.candidates().size() == 2);
        const auto idB = live.selectedCandidateId();
        const auto receiptB = QJsonDocument::fromJson(live.detailsJson().toUtf8()).object();
        if (soundLive) {
            assert(receiptA.value("schema_version").toInt() == 3);
            const auto preparation = receiptA.value("prompt_preparation").toObject();
            assert(preparation.value("original_prompt").toString() == textA);
            assert(
                preparation.value("effective_prompt")
                == receiptA.value("request").toObject().value("prompt")
            );
            if (textA == textB)
                assert(preparation == receiptB.value("prompt_preparation").toObject());
            const auto metadata = receiptA.value("request").toObject().value("metadata").toObject();
            assert(
                !metadata.contains("infer.deployment_ids")
                && !metadata.contains("infer.named_route")
            );
        }
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
        QObject::connect(&live, &GeneratedAudioController::accepted, [&](const QString& id) {
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
        assert(disclosure.contains(textA) && (textA == textB || !disclosure.contains(textB)));
        if (soundLive) {
            assert(receiptA.value("request").toObject().value("model_choice").toString() == model);
            assert(
                receiptA.value("generation_kind").toString()
                == (musicLive ? "music" : "sound_effect")
            );
            assert(!assets[0].in_memory && !assets[0].in_materials);
            const auto delivered = QString::fromStdString(
                std::string(session->session_export_source_disclosure(admitted.toStdString(), 0))
            );
            assert(delivered.contains("ai_generated"));
            const auto project = liveRoot + ".echo";
            const auto reopened = liveRoot + "-reopened";
            echo::desktop::editor_save_project(liveRoot.toStdString(), project.toStdString());
            echo::desktop::editor_open_project(project.toStdString(), reopened.toStdString());
            auto moved = echo::desktop::open_editor_session(reopened.toStdString());
            const auto restored = moved->session_list_assets();
            assert(restored.size() == 1);
            assert(
                std::string(restored[0].source_disclosure_json)
                == std::string(assets[0].source_disclosure_json)
            );
            const auto path = QString::fromStdString(std::string(restored[0].path));
            QFile restoredAudio(QDir(reopened).absoluteFilePath(path));
            assert(restoredAudio.open(QIODevice::ReadOnly));
            assert(restoredAudio.readAll() == originalBytes);
            QFile receipt(liveRoot + "/receipt.json");
            assert(receipt.open(QIODevice::WriteOnly));
            receipt.write(QJsonDocument(receiptA).toJson());
            QFile secondReceipt(liveRoot + "/second-receipt.json");
            assert(secondReceipt.open(QIODevice::WriteOnly));
            secondReceipt.write(QJsonDocument(receiptB).toJson());
        }
        QFile report(liveRoot + "/validation.json");
        assert(report.open(QIODevice::WriteOnly));
        report.write(QJsonDocument(
                         QJsonObject{
                             {"ok", true},
                             {"candidatesGenerated", 2},
                             {"candidatesAuditioned", 2},
                             {"acceptedFirst", true},
                             {"unacceptedExcluded", true},
                             {"privateProjectRoundTrip", soundLive},
                             {"acceptedOutputHash", receiptA.value("output_hash")}
                         }
        ).toJson());
    }
}
