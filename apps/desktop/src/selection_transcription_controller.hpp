//! One explicit editor request. Superseded results cannot reach the document.
#pragma once
#include <QObject>
#include <QString>
#include <cstdint>
#include <functional>

class SelectionTranscriptionController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool discarded READ discarded NOTIFY stateChanged)
    Q_PROPERTY(QString resultJson READ resultJson NOTIFY resultChanged)
    Q_PROPERTY(QString requestKey READ requestKey NOTIFY resultChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
  public:
    using TranscribeFunction = std::function<
        QString(const QString&, const QString&, const QString&, qint64, qint64, const QString&)>;
    SelectionTranscriptionController(
        QString catalog,
        QString cache,
        QObject* parent = nullptr,
        TranscribeFunction transcribe = {}
    );
    Q_INVOKABLE void request(
        const QString& id,
        qint64 start,
        qint64 end,
        const QString& endpoint,
        const QString& key
    );
    Q_INVOKABLE void discard();
    bool running() const {
        return running_;
    }
    bool discarded() const {
        return discarded_;
    }
    QString resultJson() const {
        return result_;
    }
    QString requestKey() const {
        return key_;
    }
    QString errorText() const {
        return error_;
    }
  signals:
    void stateChanged();
    void resultChanged();

  private:
    TranscribeFunction transcribe_;
    QString catalog_, cache_, result_, key_, error_;
    std::uint64_t generation_ = 0;
    bool running_ = false, discarded_ = false;
};
