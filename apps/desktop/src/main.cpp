//! Process startup: open the Library session, register the desktop backend
//! and playback controller, and load the Audio Space shell.

#include "desktop_backend.hpp"
#include "playback_controller.hpp"

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <QTimer>

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

    const std::string catalog = argc > 1 ? std::string(argv[1]) : default_catalog_path();
    const std::string cache_root = argc > 2 ? std::string(argv[2]) : default_cache_root();

    try {
        rust::Box<echo::desktop::LibrarySession> session =
            echo::desktop::open_session(catalog, cache_root);

        DesktopBackend backend(std::move(session));
        PlaybackController player;

        QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty(QStringLiteral("backend"), &backend);
        engine.rootContext()->setContextProperty(QStringLiteral("player"), &player);
        engine.loadFromModule("EchoDesktop", "Main");
        if (engine.rootObjects().isEmpty()) {
            std::cerr << "Echo QML shell failed to load" << std::endl;
            return 1;
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
        if (const char* shot = std::getenv("ECHO_DEBUG_SCREENSHOT")) {
            if (auto* window = qobject_cast<QQuickWindow*>(engine.rootObjects().first())) {
                const int delay = std::getenv("ECHO_DEBUG_AUTOPLAY") != nullptr ? 3000 : 800;
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
