#pragma once

#include "echo/audio/click_analysis.hpp"
#include <QFutureWatcher>
#include <QObject>
#include <QVariantMap>
#include <functional>
#include <optional>
#include <stop_token>

/// One bounded scanner job; newer requests supersede and serialize behind it.
class ClickAnalysisController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool cancelling READ cancelling NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
    Q_PROPERTY(QString resultKey READ resultKey NOTIFY stateChanged)
    Q_PROPERTY(QVariantMap result READ result NOTIFY stateChanged)
  public:
    using Analyze = std::function<echo::audio::ClickAnalysisResult(
        const std::string&,
        std::uint64_t,
        std::uint64_t,
        echo::audio::DeClickParameters,
        std::stop_token
    )>;
    explicit ClickAnalysisController(QObject* parent = nullptr, Analyze analyze = {});
    ~ClickAnalysisController() override;
    Q_INVOKABLE void scan(
        const QString& path,
        qint64 start,
        qint64 end,
        int sensitivity,
        int maximumMicroseconds,
        const QString& identity
    );
    Q_INVOKABLE void cancel();
    bool running() const {
        return watcher_ || pending_.has_value();
    }
    bool cancelling() const {
        return cancelling_;
    }
    QString errorText() const {
        return error_;
    }
    QString resultKey() const {
        return key_;
    }
    QVariantMap result() const {
        return result_;
    }
  signals:
    void stateChanged();

  private:
    struct Request {
        QString path;
        qint64 start;
        qint64 end;
        int sensitivity;
        int maximum;
        QString key;
        quint64 generation;
    };
    struct Outcome {
        echo::audio::ClickAnalysisResult result;
        bool success = false;
    };
    void startPending();
    Analyze analyze_;
    QFutureWatcher<Outcome>* watcher_ = nullptr;
    std::optional<Request> pending_;
    std::stop_source cancellation_;
    quint64 generation_ = 0;
    bool cancelling_ = false;
    QString error_;
    QString key_;
    QVariantMap result_;
};
