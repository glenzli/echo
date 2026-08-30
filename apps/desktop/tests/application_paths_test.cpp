#include "application_paths.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>

#include <cassert>

int main(int argc, char* argv[]) {
    QCoreApplication application(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("Echo"));
    QCoreApplication::setOrganizationName(QStringLiteral("Echo"));
    QStandardPaths::setTestModeEnabled(true);

    const ApplicationPaths paths = defaultApplicationPaths();
    const QString data_root =
        QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    const QString cache_root =
        QStandardPaths::writableLocation(QStandardPaths::CacheLocation);

    assert(QDir::isAbsolutePath(paths.catalog_path));
    assert(QDir::isAbsolutePath(paths.cache_root));
    assert(QFileInfo(paths.catalog_path).fileName() == QStringLiteral("catalog.sqlite"));
    assert(QFileInfo(paths.catalog_path).absolutePath() == QDir(data_root).absolutePath());
    assert(QDir(paths.cache_root).absolutePath() == QDir(cache_root).absolutePath());
    assert(paths.catalog_path != paths.cache_root);
}
