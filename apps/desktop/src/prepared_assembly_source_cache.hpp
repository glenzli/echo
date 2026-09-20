//! Session-owned, content-addressed rendered sources; shared by mix previews and exports.
#pragma once
#include "echo/audio/adjustment.hpp"
#include "echo/audio/offline_render.hpp"
#include <QString>
#include <QVariantMap>

class PreparedAssemblySourceCache {
  public:
    explicit PreparedAssemblySourceCache(QString root);
    struct Result {
        QString path;
        bool reused = false;
    };
    Result prepare(
        const QString& source,
        QVariantMap identity,
        const echo::audio::PlaybackAdjustment& adjustment,
        const echo::audio::OfflineRenderCallbacks& callbacks
    );
    // Call only after the mix has released all prepared sources.
    void trim(quint64 maximumBytes = 4ULL * 1024ULL * 1024ULL * 1024ULL);

  private:
    QString root_;
};
