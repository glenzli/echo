//! Compatibility inference preferences. The current defaults hard-wire the
//! local MLX environment while preserving Echo's Infer-compatible request.

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

    /// The standard HuggingFace hub cache path.
    static QString defaultModelRoot();

  signals:
    void modelRootChanged();
    void pythonChanged();
    void workerScriptChanged();

  private:
    std::unique_ptr<QSettings> settings_;
    QString model_root_;
    QString python_;
    QString worker_script_;
};
