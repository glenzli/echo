#include "inference_preferences.hpp"

#include <QDir>
#include <QStandardPaths>

namespace {

constexpr auto kRuntimeEndpointKey = "inference/runtimeEndpoint";

} // namespace

InferencePreferences::InferencePreferences(bool credentialAvailable, QObject* parent) :
    QObject(parent), credential_available_(credentialAvailable) {
    const QString config_dir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(config_dir);
    settings_ = std::make_unique<QSettings>(
        config_dir + QStringLiteral("/echo.conf"),
        QSettings::IniFormat
    );
    const QString environment_endpoint =
        QString::fromLocal8Bit(qgetenv("ECHO_INFER_ENDPOINT")).trimmed();
    runtime_endpoint_ =
        environment_endpoint.isEmpty()
            ? settings_->value(QString::fromLatin1(kRuntimeEndpointKey)).toString().trimmed()
            : environment_endpoint;
}

QString InferencePreferences::runtimeEndpoint() const {
    return runtime_endpoint_;
}

void InferencePreferences::setRuntimeEndpoint(const QString& endpoint) {
    const QString environment_endpoint =
        QString::fromLocal8Bit(qgetenv("ECHO_INFER_ENDPOINT")).trimmed();
    if (!environment_endpoint.isEmpty()) {
        runtime_endpoint_ = environment_endpoint;
        emit runtimeEndpointChanged();
        return;
    }
    const QString normalized = endpoint.trimmed();
    if (normalized == runtime_endpoint_) {
        return;
    }
    runtime_endpoint_ = normalized;
    if (runtime_endpoint_.isEmpty()) {
        settings_->remove(QString::fromLatin1(kRuntimeEndpointKey));
    } else {
        settings_->setValue(QString::fromLatin1(kRuntimeEndpointKey), runtime_endpoint_);
    }
    emit runtimeEndpointChanged();
}

bool InferencePreferences::credentialAvailable() const {
    return credential_available_;
}
