#pragma once
#include "echo/audio/audio_export.hpp"
#include <QFileInfo>
#include <QUrl>
#include <QVariantMap>
#include <cmath>
#include <stdexcept>

namespace AudioExportOptions {
inline echo::audio::AudioExportProfile profile(const QVariantMap& options) {
    echo::audio::AudioExportProfile value;
    value.format = options.value(QStringLiteral("format"), QStringLiteral("wav_pcm24"))
                       .toString()
                       .toStdString();
    const auto number = [&](const char* key, unsigned fallback) {
        bool valid = false;
        const auto raw = options.value(QString::fromLatin1(key), fallback).toDouble(&valid);
        if (!valid || !std::isfinite(raw) || raw < 0 || raw > 192000
            || raw != static_cast<unsigned>(raw))
            throw std::invalid_argument("invalid export specification");
        return static_cast<unsigned>(raw);
    };
    value.sample_rate = number("sampleRate", 48000);
    value.channels = number("channels", 2);
    value.bitrate_kbps = number("bitrateKbps", 192);
    value.validate();
    return value;
}
inline void attachMemory(echo::audio::AudioExportProfile& profile, const QVariantMap& revision) {
    const auto info = revision.value(QStringLiteral("info")).toMap();
    profile.memory_notes = info.value(QStringLiteral("notes")).toString().toStdString();
    profile.memory_place = info.value(QStringLiteral("place")).toString().toStdString();
    profile.memory_time = info.value(QStringLiteral("timeDescription")).toString().toStdString();
}
inline QString destination(const QUrl& url, const echo::audio::AudioExportProfile& profile) {
    if (!url.isLocalFile())
        throw std::invalid_argument("delivery requires a local destination");
    QString path = url.toLocalFile();
    const QString extension = QString::fromStdString(profile.extension());
    const QFileInfo info(path);
    const auto suffix = info.suffix().toLower();
    if (suffix != extension) {
        if (QStringList{"wav", "flac", "mp3", "m4a"}.contains(suffix))
            path.chop(suffix.size() + 1);
        path += QStringLiteral(".") + extension;
    }
    return QFileInfo(path).absoluteFilePath();
}
} // namespace AudioExportOptions
