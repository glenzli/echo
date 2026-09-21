//! Owns transient generation, stale-result disposal and explicit acceptance.
#pragma once
#include <QObject>
#include <QString>
#include <QUrl>
#include <functional>
#include <memory>
class QTemporaryDir;

class NarrationResult {
  public:
    virtual ~NarrationResult() = default;
    virtual QString details() const = 0;
    virtual QString accept(const QString& catalog, const QString& assembly, bool global) const = 0;
};

class GeneratedNarrationController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool accepting READ accepting NOTIFY stateChanged)
    Q_PROPERTY(QString detailsJson READ detailsJson NOTIFY stateChanged)
    Q_PROPERTY(QUrl audioUrl READ audioUrl NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
  public:
    using Generate = std::function<
        std::shared_ptr<NarrationResult>(const QString&, const QString&, const QString&)>;
    explicit GeneratedNarrationController(
        QString catalog,
        QObject* parent = nullptr,
        Generate generate = {}
    );
    Q_INVOKABLE void request(const QString& text, const QString& endpoint);
    Q_INVOKABLE void discard();
    Q_INVOKABLE void accept(const QString& assembly, bool global);
    bool running() const {
        return running_;
    }
    bool accepting() const {
        return accepting_;
    }
    QString detailsJson() const {
        return details_;
    }
    QUrl audioUrl() const;
    QString errorText() const {
        return error_;
    }
  signals:
    void stateChanged();
    void accepted(const QString& assetId);

  private:
    Generate generate_;
    QString catalog_, details_, error_;
    std::shared_ptr<NarrationResult> candidate_;
    std::shared_ptr<QTemporaryDir> directory_;
    quint64 generation_ = 0;
    bool running_ = false, accepting_ = false;
};
