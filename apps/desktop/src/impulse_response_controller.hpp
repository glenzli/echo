#pragma once

#include <QObject>
#include <QVariantList>
#include <QVariantMap>

#include <atomic>
#include <cstdint>
#include <thread>

class DesktopBackend;

/// Asynchronous local IR import. File decoding, canonical preparation, cache
/// publication and Catalog writes never block the QML/UI thread.
class ImpulseResponseController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool busy READ busy NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
    Q_PROPERTY(QVariantList impulseResponses READ impulseResponses NOTIFY stateChanged)

  public:
    explicit ImpulseResponseController(DesktopBackend& backend, QObject* parent = nullptr);
    ~ImpulseResponseController() override;

    Q_INVOKABLE void importLocalWav(
        const QString& sourcePath,
        const QString& displayName,
        const QString& creator,
        const QString& sourceUrl,
        const QString& attribution,
        const QString& rightsKind,
        const QString& spdxExpression,
        const QString& licenseUrl
    );
    Q_INVOKABLE void refresh();

    [[nodiscard]] bool busy() const;
    [[nodiscard]] QString errorText() const;
    [[nodiscard]] QVariantList impulseResponses() const;

  signals:
    void stateChanged();
    void imported(const QVariantMap& impulseResponse);

  private:
    DesktopBackend& backend_;
    std::jthread worker_;
    std::atomic<std::uint64_t> generation_{0};
    bool busy_ = false;
    QString error_text_;
    QVariantList impulse_responses_;
};
