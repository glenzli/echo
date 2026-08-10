//! Asynchronous semantic-search presentation owner.

#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>

#include <cstdint>

class SemanticSearchController : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList results READ results NOTIFY resultsChanged)
    Q_PROPERTY(QString resultsQuery READ resultsQuery NOTIFY resultsChanged)
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)

  public:
    explicit SemanticSearchController(
        QString catalogPath,
        QString runtimeEndpoint,
        QObject* parent = nullptr
    );

    Q_INVOKABLE void request(const QString& query);
    Q_INVOKABLE void clear();
    void setRuntimeEndpoint(const QString& endpoint);

    [[nodiscard]] QVariantList results() const;
    [[nodiscard]] QString resultsQuery() const;
    [[nodiscard]] bool running() const;
    [[nodiscard]] QString errorText() const;

  signals:
    void resultsChanged();
    void stateChanged();

  private:
    QString catalog_path_;
    QString runtime_endpoint_;
    QVariantList results_;
    QString results_query_;
    QString error_text_;
    std::uint64_t generation_ = 0;
    bool running_ = false;
};
