//! Session-owned, content-addressed rendered sources; shared by mix previews and exports.
#pragma once
#include "echo/audio/adjustment.hpp"
#include "echo/audio/offline_render.hpp"
#include <QSet>
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
    // Pinned sources may still be read by a streaming preview, including later clips.
    void trim(
        quint64 maximumBytes = 4ULL * 1024ULL * 1024ULL * 1024ULL,
        const QSet<QString>& pinnedPaths = {}
    );

  private:
    QString root_;
};
