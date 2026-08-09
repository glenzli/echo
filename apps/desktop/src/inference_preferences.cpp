#include "inference_preferences.hpp"

#include <QDir>
#include <QStandardPaths>

namespace {

constexpr auto kRuntimeEndpointKey = "inference/runtimeEndpoint";
constexpr auto kDefaultRuntimeEndpoint = "http://127.0.0.1:8787";

} // namespace

InferencePreferences::InferencePreferences(bool credentialAvailable, QObject* parent) :
    QObject(parent), credential_available_(credentialAvailable) {
    const QString config_dir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(config_dir);
    settings_ = std::make_unique<QSettings>(
        config_dir + QStringLiteral("/echo.conf"),
        QSettings::IniFormat
    );
    const QString environment_endpoint = QString::fromLocal8Bit(qgetenv("ECHO_INFER_ENDPOINT"));
    runtime_endpoint_ =
        settings_
            ->value(
                QString::fromLatin1(kRuntimeEndpointKey),
                environment_endpoint.isEmpty() ? QString::fromLatin1(kDefaultRuntimeEndpoint)
                                               : environment_endpoint
            )
            .toString();
}

QString InferencePreferences::runtimeEndpoint() const {
    return runtime_endpoint_;
}

void InferencePreferences::setRuntimeEndpoint(const QString& endpoint) {
    const QString normalized = endpoint.trimmed();
    if (normalized.isEmpty() || normalized == runtime_endpoint_) {
        return;
    }
    runtime_endpoint_ = normalized;
    settings_->setValue(QString::fromLatin1(kRuntimeEndpointKey), runtime_endpoint_);
    emit runtimeEndpointChanged();
}

bool InferencePreferences::credentialAvailable() const {
    return credential_available_;
}
