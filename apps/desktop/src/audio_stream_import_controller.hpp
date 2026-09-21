#pragma once
#include <QObject>
#include <QUrl>
#include <QVariantList>
#include <atomic>
#include <memory>
// Owns bounded asynchronous intake inspection, explicit stream selection and preserved containers.
class AudioStreamImportController final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool busy READ busy NOTIFY stateChanged)
    Q_PROPERTY(bool choosing READ choosing NOTIFY stateChanged)
    Q_PROPERTY(QVariantList streams READ streams NOTIFY stateChanged)
    Q_PROPERTY(QString sourceName READ sourceName NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
  public:
    explicit AudioStreamImportController(QString storageRoot, QObject* parent = nullptr);
    ~AudioStreamImportController() override;
    Q_INVOKABLE void inspect(const QList<QUrl>& files);
    Q_INVOKABLE void select(int streamIndex);
    Q_INVOKABLE void cancel();
    bool busy() const {
        return busy_;
    }
    bool choosing() const {
        return choosing_;
    }
    QVariantList streams() const {
        return streams_;
    }
    QString sourceName() const;
    QString errorText() const {
        return error_;
    }
  signals:
    void stateChanged();
    void filesReady(const QList<QUrl>& files);

  private:
    void next();
    QString root_, error_;
    QList<QUrl> files_, ready_;
    QVariantList streams_;
    qsizetype cursor_ = 0;
    bool busy_ = false, choosing_ = false;
    std::shared_ptr<std::atomic<bool>> cancelled_;
};
