#pragma once

#include <QFutureWatcher>
#include <QObject>
#include <QVariantMap>
#include <optional>
#include <stop_token>

/// One bounded, cancellable noise-learning job, with latest-request delivery.
class NoiseProfileController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(int error READ error NOTIFY stateChanged)
  public:
    explicit NoiseProfileController(QObject* parent = nullptr);
    ~NoiseProfileController() override;
    Q_INVOKABLE void
    capture(const QString& path, const QString& identity, qint64 startMillis, qint64 endMillis);
    Q_INVOKABLE void cancel();
    [[nodiscard]] bool running() const {
        return running_;
    }
    [[nodiscard]] int error() const {
        return error_;
    }
  signals:
    void stateChanged();
    void profileReady(const QString& identity, const QVariantMap& profile);

  private:
    struct Request {
        QString path;
        QString identity;
        qint64 start;
        qint64 end;
        quint64 generation;
    };
    void startPending();
    QFutureWatcher<QVariantMap>* watcher_ = nullptr;
    std::optional<Request> pending_;
    std::stop_source cancellation_;
    quint64 generation_ = 0;
    bool running_ = false;
    int error_ = 0;
};
