//! Canonical user-writable storage locations for a no-argument desktop
//! launch. Explicit command-line paths remain available to development tools.

#pragma once

#include <QString>

struct ApplicationPaths {
    QString catalog_path;
    QString cache_root;
};

ApplicationPaths defaultApplicationPaths();
