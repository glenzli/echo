//! Process startup: open the Library session, register the desktop backend,
//! playback controller, and UI preferences, then load the Audio Space shell.

#include "batch_export_controller.hpp"
#include "desktop_backend.hpp"
#include "impulse_response_controller.hpp"
#include "inference_preferences.hpp"
#include "loudness_analysis_controller.hpp"
#include "playback_controller.hpp"
#include "render_export_controller.hpp"
#include "semantic_search_controller.hpp"
#include "ui_preferences.hpp"

#if defined(Q_OS_MACOS)
#include "mac_titlebar.hpp"
#endif

#include <QGuiApplication>
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

namespace {

std::string default_catalog_path() {
    return "catalogs/demo.sqlite";
}

std::string default_cache_root() {
    return "cache";
}

} // namespace

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

    const std::string catalog = argc > 1 ? std::string(argv[1]) : default_catalog_path();
    const std::string cache_root = argc > 2 ? std::string(argv[2]) : default_cache_root();

    try {
        rust::Box<echo::desktop::LibrarySession> session =
            echo::desktop::open_session(catalog, cache_root);

        DesktopBackend backend(std::move(session));
        PlaybackController player;
        LoudnessAnalysisController loudness_analyzer;
        RenderExportController render_exporter(backend);
        BatchExportController batch_exporter(backend);
        ImpulseResponseController impulse_response_controller(backend);
        UiPreferences ui_prefs(application);
        InferencePreferences inference_prefs(echo::desktop::infer_runtime_credential_available());
        SemanticSearchController semantic_search(
            QString::fromStdString(catalog),
            inference_prefs.runtimeEndpoint()
        );
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
        engine.rootContext()->setContextProperty(QStringLiteral("player"), &player);
        engine.rootContext()->setContextProperty(
            QStringLiteral("loudnessAnalyzer"),
            &loudness_analyzer
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("renderExporter"),
            &render_exporter
        );
        engine.rootContext()->setContextProperty(QStringLiteral("batchExporter"), &batch_exporter);
        engine.rootContext()->setContextProperty(
            QStringLiteral("impulseResponseController"),
            &impulse_response_controller
        );
        engine.rootContext()->setContextProperty(QStringLiteral("uiPrefs"), &ui_prefs);
        engine.rootContext()->setContextProperty(
            QStringLiteral("inferencePrefs"),
            &inference_prefs
        );
        engine.rootContext()->setContextProperty(
            QStringLiteral("semanticSearch"),
            &semantic_search
        );
        ui_prefs.attachEngine(engine);
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
        if (std::getenv("ECHO_DEBUG_OPEN_EDITOR") != nullptr) {
            QObject* root = engine.rootObjects().first();
            QTimer::singleShot(750, root, [root] {
                QMetaObject::invokeMethod(root, "showSoundEditor");
            });
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
                                     || std::getenv("ECHO_DEBUG_OPEN_EDITOR") != nullptr
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
