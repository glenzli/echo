//! Process startup: open the Library session, register the desktop backend,
//! and load the Audio Space shell.

#include "desktop_backend.hpp"

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

} // namespace

int main(int argc, char* argv[]) {
    QGuiApplication application(argc, argv);
    QGuiApplication::setApplicationName(QStringLiteral("Echo"));
    QGuiApplication::setOrganizationName(QStringLiteral("Echo"));
    qputenv("QT_QUICK_CONTROLS_STYLE", "Basic");

    const std::string catalog = argc > 1 ? std::string(argv[1]) : default_catalog_path();

    try {
        rust::Box<echo::desktop::LibrarySession> session = echo::desktop::open_session(catalog);

        DesktopBackend backend(std::move(session));

        QQmlApplicationEngine engine;
        engine.rootContext()->setContextProperty(QStringLiteral("backend"), &backend);
        engine.loadFromModule("EchoDesktop", "Main");
        if (engine.rootObjects().isEmpty()) {
            std::cerr << "Echo QML shell failed to load" << std::endl;
            return 1;
        }
        // Headless smoke aid: ECHO_DEBUG_SCREENSHOT=/path.png captures the
        // first window after the shell settles, then exits.
        if (const char* shot = std::getenv("ECHO_DEBUG_SCREENSHOT")) {
            if (auto* window = qobject_cast<QQuickWindow*>(engine.rootObjects().first())) {
                QTimer::singleShot(800, window, [window, shot] {
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
