//! Infer Runtime consumer preferences. The endpoint is ordinary product
//! configuration. Credential storage belongs to the Rust security owner; this
//! presentation object receives only its availability state.

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
    explicit InferencePreferences(bool credentialAvailable, QObject* parent = nullptr);

    QString runtimeEndpoint() const;
    void setRuntimeEndpoint(const QString& endpoint);
    bool credentialAvailable() const;

  signals:
    void runtimeEndpointChanged();

  private:
    std::unique_ptr<QSettings> settings_;
    QString runtime_endpoint_;
    bool credential_available_ = false;
};
