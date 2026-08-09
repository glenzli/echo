//! Infer Runtime consumer preferences. The endpoint is ordinary product
//! configuration; the bearer token is process-secret input and is never
//! exposed as a Qt property or persisted by this owner.

#pragma once

#include <QObject>
#include <QSettings>
#include <QString>

#include <memory>

class InferencePreferences : public QObject {
    Q_OBJECT
    Q_PROPERTY(
        QString runtimeEndpoint READ runtimeEndpoint WRITE setRuntimeEndpoint NOTIFY
            runtimeEndpointChanged
    )
    Q_PROPERTY(bool credentialAvailable READ credentialAvailable CONSTANT)

  public:
    explicit InferencePreferences(QObject* parent = nullptr);

    QString runtimeEndpoint() const;
    void setRuntimeEndpoint(const QString& endpoint);
    bool credentialAvailable() const;

    /// Returns the process-injected consumer token to native startup only.
    QString runtimeToken() const;

  signals:
    void runtimeEndpointChanged();

  private:
    std::unique_ptr<QSettings> settings_;
    QString runtime_endpoint_;
};
