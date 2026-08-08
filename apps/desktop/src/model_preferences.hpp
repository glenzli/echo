//! Model preferences: where Echo finds models and the MLX worker. Echo never
//! downloads; the user maintains the shared HF cache.

#pragma once

#include <QObject>
#include <QSettings>
#include <QString>

#include <memory>

class ModelPreferences : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString modelRoot READ modelRoot WRITE setModelRoot NOTIFY modelRootChanged)
    Q_PROPERTY(QString python READ python WRITE setPython NOTIFY pythonChanged)
    Q_PROPERTY(
        QString workerScript READ workerScript WRITE setWorkerScript NOTIFY workerScriptChanged
    )

  public:
    explicit ModelPreferences(QObject* parent = nullptr);

    QString modelRoot() const;
    void setModelRoot(const QString& root);

    QString python() const;
    void setPython(const QString& python);

    QString workerScript() const;
    void setWorkerScript(const QString& script);

    QString ollamaEndpoint() const;
    void setOllamaEndpoint(const QString& endpoint);

    QString ollamaModel() const;
    void setOllamaModel(const QString& model);

    /// The standard HuggingFace hub cache path.
    static QString defaultModelRoot();

  signals:
    void modelRootChanged();
    void pythonChanged();
    void workerScriptChanged();
    void ollamaEndpointChanged();
    void ollamaModelChanged();

  private:
    std::unique_ptr<QSettings> settings_;
    QString model_root_;
    QString python_;
    QString worker_script_;
    QString ollama_endpoint_;
    QString ollama_model_;
};
