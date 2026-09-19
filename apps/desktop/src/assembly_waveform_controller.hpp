//! Asynchronous, bounded source waveform projection for the arrangement editor.
#pragma once

#include <QObject>
#include <QSet>
#include <QThreadPool>
#include <QVariantMap>

class AssemblyWaveformController : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap waveforms READ waveforms NOTIFY waveformsChanged)

  public:
    AssemblyWaveformController(QString catalogPath, QString cacheRoot, QObject* parent = nullptr);
    ~AssemblyWaveformController() override;
    Q_INVOKABLE void setSources(const QStringList& assetIds);
    [[nodiscard]] QVariantMap waveforms() const;

  signals:
    void waveformsChanged();

  private:
    void requestNext();
    QString catalog_path_;
    QString cache_root_;
    QThreadPool pool_;
    QSet<QString> wanted_;
    QSet<QString> pending_;
    QVariantMap waveforms_;
};
