#include "application_paths.hpp"

#include <QDir>
#include <QStandardPaths>

#include <stdexcept>

ApplicationPaths defaultApplicationPaths() {
    const QString data_root = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    const QString cache_root = QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
    if (data_root.isEmpty() || cache_root.isEmpty()) {
        throw std::runtime_error("macOS did not provide user-writable Echo data locations");
    }
    return {
        .catalog_path = QDir(data_root).filePath(QStringLiteral("catalog.sqlite")),
        .cache_root = cache_root,
    };
}
