//! Process startup: open the Library session, register the desktop backend,
//! playback controller, and UI preferences, then load the Audio Space shell.

#include "application_paths.hpp"
#include "assembly_waveform_controller.hpp"
#include "batch_export_controller.hpp"
#include "creative_vfx_presets.hpp"
#include "desktop_backend.hpp"
#include "impulse_response_controller.hpp"
#include "inference_preferences.hpp"
#include "loudness_analysis_controller.hpp"
#include "noise_profile_controller.hpp"
#include "playback_controller.hpp"
#include "render_export_controller.hpp"
#include "rendered_spectral_working_copy_controller.hpp"
#include "semantic_search_controller.hpp"
#include "sound_assembly_controller.hpp"
#include "spectrogram_preview_controller.hpp"
#include "ui_preferences.hpp"

#if defined(Q_OS_MACOS)
#include "mac_titlebar.hpp"
#endif

#include <QFile>
#include <QGuiApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickWindow>
#include <QTimer>
#include <QUrl>

#include <cstdlib>
#include <iostream>
#include <string>

#include "echo-desktop-bridge/src/lib.rs.h"
#include "rust/cxx.h"

int main(int argc, char* argv[]) {
    QGuiApplication application(argc, argv);
    QGuiApplication::setApplicationName(QStringLiteral("Echo"));
    QGuiApplication::setOrganizationName(QStringLiteral("Echo"));
    qputenv("QT_QUICK_CONTROLS_STYLE", "Basic");
    qmlRegisterUncreatableType<UiPreferences>(
        "EchoDesktop",
        0,
        1,
        "UiPreferences",
        "UiPreferences is created by the host application"
    );
    qmlRegisterUncreatableType<InferencePreferences>(
        "EchoDesktop",
        0,
        1,
        "InferencePreferences",
        "InferencePreferences is created by the host application"
    );

    const ApplicationPaths default_paths = defaultApplicationPaths();
    const std::string catalog =
        argc > 1 ? std::string(argv[1]) : default_paths.catalog_path.toStdString();
    const std::string cache_root =
        argc > 2 ? std::string(argv[2]) : default_paths.cache_root.toStdString();

    try {
        rust::Box<echo::desktop::LibrarySession> session =
            echo::desktop::open_session(catalog, cache_root);

        DesktopBackend backend(std::move(session));
        PlaybackController player;
        PlaybackController material_player;
        SoundAssemblyController sound_assembly(backend, player);
        AssemblyWaveformController assembly_waveforms(
            QString::fromStdString(catalog),
            QString::fromStdString(cache_root)
        );
        LoudnessAnalysisController loudness_analyzer;
        RenderExportController render_exporter(backend);
        RenderedSpectralWorkingCopyController rendered_spectral_working_copy(backend);
        BatchExportController batch_exporter(backend);
        ImpulseResponseController impulse_response_controller(backend);
        UiPreferences ui_prefs(application);
        CreativeVfxPresets creative_vfx_presets;
        InferencePreferences inference_prefs(echo::desktop::infer_runtime_credential_available());
        SemanticSearchController semantic_search(
            QString::fromStdString(catalog),
            inference_prefs.runtimeEndpoint()
        );
        SpectrogramPreviewController spectrogram_preview;
        NoiseProfileController noise_profile;
        QObject::connect(
            &inference_prefs,
            &InferencePreferences::runtimeEndpointChanged,
            &semantic_search,
            [&inference_prefs, &semantic_search] {
                semantic_search.setRuntimeEndpoint(inference_prefs.runtimeEndpoint());
            }
        );
        backend.startWorkers(inference_prefs.runtimeEndpoint());

        QQmlApplicationEngine engine;
        // Qt 6.11's default import paths start at qrc:/qt/qml, while Echo's
        // executable module keeps its generated qmldir at qrc:/EchoDesktop.
        engine.addImportPath(QStringLiteral("qrc:/"));
        engine.rootContext()->setContextProperty(QStringLiteral("backend"), &backend);
        engine.rootContext()->setContextProperty(
            QStringLiteral("assemblyWaveforms"),
            &assembly_waveforms
        );
        engine.rootContext()->setContextProperty(QStringLiteral("player"), &player);
        engine.rootContext()->setContextProperty(QStringLiteral("noiseProfile"), &noise_profile);
        engine.rootContext()->setContextProperty(
            QStringLiteral("materialPlayer"),
            &material_player
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("soundAssemblyController"),
            &sound_assembly
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("loudnessAnalyzer"),
            &loudness_analyzer
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("renderExporter"),
            &render_exporter
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("renderedSpectralWorkingCopy"),
            &rendered_spectral_working_copy
        );
        engine.rootContext()->setContextProperty(QStringLiteral("batchExporter"), &batch_exporter);
        engine.rootContext()->setContextProperty(
            QStringLiteral("impulseResponseController"),
            &impulse_response_controller
        );
        engine.rootContext()->setContextProperty(QStringLiteral("uiPrefs"), &ui_prefs);
        engine.rootContext()->setContextProperty(
            QStringLiteral("creativeVfxPresets"),
            &creative_vfx_presets
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("inferencePrefs"),
            &inference_prefs
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("semanticSearch"),
            &semantic_search
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("spectrogramPreview"),
            &spectrogram_preview
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("memorySmokeMaterial"),
            qEnvironmentVariable("ECHO_DEBUG_MEMORY_MATERIAL")
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("multitrackSmokeRoot"),
            qEnvironmentVariable("ECHO_DEBUG_MULTITRACK_ROOT")
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("complexSmokeEnabled"),
            qEnvironmentVariableIsSet("ECHO_DEBUG_COMPLEX")
        );
        ui_prefs.attachEngine(engine);
        engine.rootContext()->setContextProperty(
            QStringLiteral("spectralSmokeRoot"),
            qEnvironmentVariable("ECHO_DEBUG_SPECTRAL_ROOT")
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("noiseSmokeRoot"),
            qEnvironmentVariable("ECHO_DEBUG_NOISE_ROOT")
        );
        engine.loadFromModule("EchoDesktop", "Main");
        if (engine.rootObjects().isEmpty()) {
            std::cerr << "Echo QML shell failed to load" << std::endl;
            return 1;
        }
#if defined(Q_OS_MACOS)
        QObject* const root_object = engine.rootObjects().first();
        QObject* const title_toolbar =
            root_object->findChild<QObject*>(QStringLiteral("titleToolBar"));
        const int title_bar_height =
            title_toolbar == nullptr ? 48 : qRound(title_toolbar->property("height").toReal());
        installMacTitleBarAlignment(qobject_cast<QQuickWindow*>(root_object), title_bar_height);
#endif
        // Optional fixture-driven memory workflow. Readiness comes from QML and
        // real catalog/render results; each distinct page is captured once.
        const bool noise_smoke = !qEnvironmentVariable("ECHO_DEBUG_NOISE_REPORT").isEmpty();
        const bool spectral_smoke = !qEnvironmentVariable("ECHO_DEBUG_SPECTRAL_REPORT").isEmpty();
        const bool multitrack_smoke =
            !qEnvironmentVariable("ECHO_DEBUG_MULTITRACK_REPORT").isEmpty();
        if (const auto report_path = qEnvironmentVariable(
                noise_smoke        ? "ECHO_DEBUG_NOISE_REPORT"
                : spectral_smoke   ? "ECHO_DEBUG_SPECTRAL_REPORT"
                : multitrack_smoke ? "ECHO_DEBUG_MULTITRACK_REPORT"
                                   : "ECHO_DEBUG_MEMORY_REPORT"
            );
            !report_path.isEmpty()) {
            auto* root = engine.rootObjects().first();
            auto* timer = new QTimer(root);
            timer->setInterval(150);
            QObject::connect(
                timer,
                &QTimer::timeout,
                root,
                [root,
                 report_path,
                 multitrack_smoke,
                 spectral_smoke,
                 noise_smoke,
                 last_stage = -1]() mutable {
                    const int stage = root->property(
                                              noise_smoke        ? "noiseSmokeStage"
                                              : spectral_smoke   ? "spectralSmokeStage"
                                              : multitrack_smoke ? "multitrackSmokeStage"
                                                                 : "memorySmokeStage"
                    )
                                          .toInt();
                    if (stage != last_stage) {
                        if (auto* window = qobject_cast<QQuickWindow*>(root))
                            window->grabWindow().save(
                                report_path + QStringLiteral(".%1.png").arg(stage)
                            );
                        last_stage = stage;
                    }
                    const auto report = root->property(
                                                noise_smoke        ? "noiseSmokeReport"
                                                : spectral_smoke   ? "spectralSmokeReport"
                                                : multitrack_smoke ? "multitrackSmokeReport"
                                                                   : "memorySmokeReport"
                    )
                                            .toString()
                                            .toUtf8();
                    if (report.isEmpty())
                        return;
                    if (spectral_smoke || noise_smoke) {
                        if (auto* window = qobject_cast<QQuickWindow*>(root))
                            window->grabWindow().save(
                                report_path + QStringLiteral(".complete.png")
                            );
                    }
                    QFile file(report_path);
                    const bool written =
                        file.open(QIODevice::WriteOnly) && file.write(report) == report.size();
                    const bool passed = QJsonDocument::fromJson(report)
                                            .object()
                                            .value(QStringLiteral("ok"))
                                            .toBool();
                    QGuiApplication::exit(written && passed ? 0 : 1);
                }
            );
            timer->start();
            QTimer::singleShot(90'000, &application, [] { QGuiApplication::exit(2); });
        }
        // Headless smoke aids: ECHO_DEBUG_SCREENSHOT=/path.png captures the
        // first window after the shell settles; ECHO_DEBUG_AUTOPLAY=/file.wav
        // plays a recording first (used together for automated playback
        // smoke). Both are dev-only.
        if (const char* autoplay = std::getenv("ECHO_DEBUG_AUTOPLAY")) {
            const QString autoplay_path = QString::fromUtf8(autoplay);
            QTimer::singleShot(500, &player, [&player, autoplay_path] {
                qInfo("autoplay: %s", qPrintable(autoplay_path));
                player.play(autoplay_path);
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_SETTINGS") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(250, root, [root] {
                QMetaObject::invokeMethod(root, "openSettings");
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_LIBRARY") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(250, root, [root] {
                QMetaObject::invokeMethod(root, "openLibrary");
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_ALBUM") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root] {
                QMetaObject::invokeMethod(root, "debugOpenAlbumDialog");
            });
        }
        if (const char* album_name = std::getenv("ECHO_DEBUG_CREATE_ALBUM")) {
            QObject* root = engine.rootObjects().first();
            const QString name = QString::fromUtf8(album_name);
            QTimer::singleShot(900, root, [root, name] {
                QMetaObject::invokeMethod(root, "debugCreateAlbum", Q_ARG(QString, name));
            });
        }
        if (const char* search_text = std::getenv("ECHO_DEBUG_SEARCH")) {
            QObject* root = engine.rootObjects().first();
            const QString query = QString::fromUtf8(search_text);
            QTimer::singleShot(600, root, [root, query] {
                QMetaObject::invokeMethod(root, "debugSearch", Q_ARG(QString, query));
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_TAPE") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root] {
                QMetaObject::invokeMethod(root, "debugOpenSoundTape");
            });
        }
        if (std::getenv("ECHO_DEBUG_PLAY_TAPE") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(1'200, root, [root] {
                QMetaObject::invokeMethod(root, "debugPlaySoundTape");
            });
        }
        if (const auto path = qEnvironmentVariable("ECHO_DEBUG_SPECTRAL_SOURCE"); !path.isEmpty()) {
            auto* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root, path] {
                QMetaObject::invokeMethod(root, "debugOpenSpectralSource", Q_ARG(QString, path));
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_EDITOR") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root] {
                QMetaObject::invokeMethod(root, "showSoundEditor");
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_ASSEMBLY") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root] {
                QMetaObject::invokeMethod(root, "showSoundAssembly");
            });
        }
        if (std::getenv("ECHO_DEBUG_CREATE_ASSEMBLY") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(900, root, [root] {
                QMetaObject::invokeMethod(root, "debugCreateSoundAssembly");
            });
        }
        if (const char* assembly_export_path = std::getenv("ECHO_DEBUG_ASSEMBLY_EXPORT")) {
            QObject* root = engine.rootObjects().first();
            const QUrl destination = QUrl::fromLocalFile(QString::fromUtf8(assembly_export_path));
            QObject::connect(
                &sound_assembly,
                &SoundAssemblyController::stateChanged,
                &application,
                [&sound_assembly] {
                    if (!sound_assembly.running()
                        && (sound_assembly.hasResult() || !sound_assembly.errorText().isEmpty())) {
                        QGuiApplication::exit(sound_assembly.hasResult() ? 0 : 1);
                    }
                }
            );
            QTimer::singleShot(1'200, root, [root, destination] {
                QMetaObject::invokeMethod(
                    root,
                    "debugExportSoundAssembly",
                    Q_ARG(QUrl, destination)
                );
            });
            QTimer::singleShot(120'000, &application, [] { QGuiApplication::exit(2); });
        }
        if (std::getenv("ECHO_DEBUG_REPLAY_EDITOR") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root] {
                QMetaObject::invokeMethod(root, "debugReplaySoundEditor");
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_EXPORT") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(900, root, [root] {
                QMetaObject::invokeMethod(root, "debugOpenExportDialog");
            });
        }
        if (std::getenv("ECHO_DEBUG_OPEN_BATCH_EXPORT") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(900, root, [root] {
                QMetaObject::invokeMethod(root, "debugOpenBatchDialog");
            });
        }
        if (const char* batch_directory = std::getenv("ECHO_DEBUG_BATCH_EXPORT")) {
            QObject* root = engine.rootObjects().first();
            const QUrl destination = QUrl::fromLocalFile(QString::fromUtf8(batch_directory));
            const QString format = QString::fromUtf8(
                std::getenv("ECHO_DEBUG_BATCH_FORMAT") == nullptr
                    ? "wav_pcm24"
                    : std::getenv("ECHO_DEBUG_BATCH_FORMAT")
            );
            QObject::connect(
                &batch_exporter,
                &BatchExportController::stateChanged,
                &application,
                [&batch_exporter] {
                    if (!batch_exporter.running() && batch_exporter.hasResult()) {
                        QGuiApplication::exit(batch_exporter.failedCount() == 0 ? 0 : 1);
                    }
                }
            );
            QTimer::singleShot(900, root, [root, destination, format] {
                QMetaObject::invokeMethod(
                    root,
                    "debugBatchExport",
                    Q_ARG(QUrl, destination),
                    Q_ARG(QString, format)
                );
            });
            QTimer::singleShot(120'000, &application, [] { QGuiApplication::exit(2); });
        }
        if (std::getenv("ECHO_DEBUG_RESUME_BATCH_EXPORT") != nullptr) {
            QObject::connect(
                &batch_exporter,
                &BatchExportController::stateChanged,
                &application,
                [&batch_exporter] {
                    if (!batch_exporter.running() && batch_exporter.hasResult()) {
                        QGuiApplication::exit(batch_exporter.failedCount() == 0 ? 0 : 1);
                    }
                }
            );
            QTimer::singleShot(900, &batch_exporter, [&batch_exporter] {
                batch_exporter.resume();
            });
            QTimer::singleShot(120'000, &application, [] { QGuiApplication::exit(2); });
        }
        if (const char* export_path = std::getenv("ECHO_DEBUG_EXPORT")) {
            QObject* root = engine.rootObjects().first();
            const QUrl destination = QUrl::fromLocalFile(QString::fromUtf8(export_path));
            QObject::connect(
                &render_exporter,
                &RenderExportController::stateChanged,
                &application,
                [&render_exporter] {
                    if (!render_exporter.running()
                        && (render_exporter.hasResult()
                            || !render_exporter.errorText().isEmpty())) {
                        QGuiApplication::exit(render_exporter.hasResult() ? 0 : 1);
                    }
                }
            );
            QTimer::singleShot(900, root, [root, destination] {
                QMetaObject::invokeMethod(root, "debugExportSound", Q_ARG(QUrl, destination));
            });
            QTimer::singleShot(30'000, &application, [] { QGuiApplication::exit(2); });
        }
        if (const char* shot = std::getenv("ECHO_DEBUG_SCREENSHOT")) {
            if (auto* window = qobject_cast<QQuickWindow*>(engine.rootObjects().first())) {
                const bool replay_editor = std::getenv("ECHO_DEBUG_REPLAY_EDITOR") != nullptr;
                const bool delayed = std::getenv("ECHO_DEBUG_AUTOPLAY") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_SETTINGS") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_LIBRARY") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_ALBUM") != nullptr
                                     || std::getenv("ECHO_DEBUG_CREATE_ALBUM") != nullptr
                                     || std::getenv("ECHO_DEBUG_SEARCH") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_TAPE") != nullptr
                                     || std::getenv("ECHO_DEBUG_PLAY_TAPE") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_EDITOR") != nullptr
                                     || std::getenv("ECHO_DEBUG_SPECTRAL_SOURCE") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_ASSEMBLY") != nullptr
                                     || std::getenv("ECHO_DEBUG_CREATE_ASSEMBLY") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_EXPORT") != nullptr
                                     || std::getenv("ECHO_DEBUG_OPEN_BATCH_EXPORT") != nullptr
                                     || replay_editor;
                const bool semantic_search = std::getenv("ECHO_DEBUG_SEARCH") != nullptr;
                const int delay = replay_editor     ? 5000
                                  : semantic_search ? 6000
                                  : delayed         ? 3000
                                                    : 800;
                QTimer::singleShot(delay, window, [window, shot] {
                    window->grabWindow().save(QString::fromUtf8(shot));
                    QGuiApplication::exit(0);
                });
            }
        }
        return QGuiApplication::exec();
    } catch (const rust::Error& error) {
        std::cerr << "cannot open catalog " << catalog << ": " << error.what() << std::endl;
        return 1;
    }
}
