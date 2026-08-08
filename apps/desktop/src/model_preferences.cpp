#include "model_preferences.hpp"

#include <QDir>
#include <QStandardPaths>

namespace {

constexpr auto kModelRootKey = "models/root";
constexpr auto kPythonKey = "models/python";
constexpr auto kWorkerKey = "models/worker";
constexpr auto kOllamaEndpointKey = "models/ollamaEndpoint";
constexpr auto kOllamaModelKey = "models/ollamaModel";

} // namespace

ModelPreferences::ModelPreferences(QObject* parent) : QObject(parent) {
    const QString config_dir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(config_dir);
    settings_ = std::make_unique<QSettings>(
        config_dir + QStringLiteral("/echo.conf"),
        QSettings::IniFormat
    );
    model_root_ =
        settings_->value(QString::fromLatin1(kModelRootKey), defaultModelRoot()).toString();
    python_ = settings_
                  ->value(
                      QString::fromLatin1(kPythonKey),
                      QString::fromLocal8Bit(qgetenv("ECHO_MLX_PYTHON"))
                  )
                  .toString();
    if (python_.isEmpty()) {
        python_ = QStringLiteral("python3");
    }
    worker_script_ = settings_
                         ->value(
                             QString::fromLatin1(kWorkerKey),
                             QString::fromLocal8Bit(qgetenv("ECHO_ASR_WORKER"))
                         )
                         .toString();
    if (worker_script_.isEmpty()) {
        worker_script_ = QStringLiteral("tools/asr/transcribe.py");
    }
}

QString ModelPreferences::modelRoot() const {
    return model_root_;
}

void ModelPreferences::setModelRoot(const QString& root) {
    if (root == model_root_) {
        return;
    }
    model_root_ = root;
    settings_->setValue(QString::fromLatin1(kModelRootKey), model_root_);
    emit modelRootChanged();
}

QString ModelPreferences::python() const {
    return python_;
}

void ModelPreferences::setPython(const QString& python) {
    if (python == python_) {
        return;
    }
    python_ = python;
    settings_->setValue(QString::fromLatin1(kPythonKey), python_);
    emit pythonChanged();
}

QString ModelPreferences::workerScript() const {
    return worker_script_;
}

void ModelPreferences::setWorkerScript(const QString& script) {
    if (script == worker_script_) {
        return;
    }
    worker_script_ = script;
    settings_->setValue(QString::fromLatin1(kWorkerKey), worker_script_);
    emit workerScriptChanged();
}

QString ModelPreferences::ollamaEndpoint() const {
    return ollama_endpoint_;
}

void ModelPreferences::setOllamaEndpoint(const QString& endpoint) {
    if (endpoint == ollama_endpoint_) {
        return;
    }
    ollama_endpoint_ = endpoint;
    settings_->setValue(QString::fromLatin1(kOllamaEndpointKey), ollama_endpoint_);
    emit ollamaEndpointChanged();
}

QString ModelPreferences::ollamaModel() const {
    return ollama_model_;
}

void ModelPreferences::setOllamaModel(const QString& model) {
    if (model == ollama_model_) {
        return;
    }
    ollama_model_ = model;
    settings_->setValue(QString::fromLatin1(kOllamaModelKey), ollama_model_);
    emit ollamaModelChanged();
}

QString ModelPreferences::defaultModelRoot() {
    const QString hf_home = QString::fromLocal8Bit(qgetenv("HF_HOME"));
    if (!hf_home.isEmpty()) {
        return hf_home;
    }
    const QString hf_cache = QString::fromLocal8Bit(qgetenv("HF_HUB_CACHE"));
    if (!hf_cache.isEmpty()) {
        return hf_cache;
    }
    return QDir::homePath() + QStringLiteral("/.cache/huggingface/hub");
}
