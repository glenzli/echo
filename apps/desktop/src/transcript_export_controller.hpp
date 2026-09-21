//! Explicit delivery of recognized text and original-time subtitle evidence.
#pragma once
#include <QObject>
#include <QUrl>
#include <QVariantMap>

class TranscriptExportController final : public QObject {
    Q_OBJECT
  public:
    using QObject::QObject;
    Q_INVOKABLE bool hasTiming(const QVariantMap& record) const;
    Q_INVOKABLE QString copyText(const QVariantMap& record) const;
    Q_INVOKABLE QString save(
        const QVariantMap& record,
        const QUrl& destination,
        const QString& format,
        const QString& sourcePath
    ) const;
};
