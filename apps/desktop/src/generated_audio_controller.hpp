//! Owns transient generation, stale-result disposal and explicit acceptance.
#pragma once
#include <QObject>
#include <QString>
#include <QUrl>
#include <QVariantList>
#include <functional>
#include <memory>
#include <vector>
class QTemporaryDir;

class GeneratedAudioResult {
  public:
    virtual ~GeneratedAudioResult() = default;
    virtual QString details() const = 0;
    virtual QString accept(const QString& catalog, const QString& assembly, bool global) const = 0;
};

struct GeneratedAudioRequest {
    QString text;
    int durationSeconds = 0;
    quint32 seed = 0;
    bool ambience = false;
    QString model = QStringLiteral("stable_audio_3_small_sfx");
    QString preparation = {};
};

class GeneratedAudioController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY stateChanged)
    Q_PROPERTY(bool accepting READ accepting NOTIFY stateChanged)
    Q_PROPERTY(bool preparing READ preparing NOTIFY stateChanged)
    Q_PROPERTY(bool stopping READ stopping NOTIFY stateChanged)
    Q_PROPERTY(QString detailsJson READ detailsJson NOTIFY stateChanged)
    Q_PROPERTY(QUrl audioUrl READ audioUrl NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
    Q_PROPERTY(QVariantList candidates READ candidates NOTIFY stateChanged)
    Q_PROPERTY(QString selectedCandidateId READ selectedCandidateId NOTIFY stateChanged)
  public:
    enum class Kind { Narration, SoundMaterial };
    using Generate = std::function<std::shared_ptr<
        GeneratedAudioResult>(const GeneratedAudioRequest&, const QString&, const QString&)>;
    using Prepare = std::function<QString(const QString&, const QString&)>;
    explicit GeneratedAudioController(
        QString catalog,
        QObject* parent = nullptr,
        Generate generate = {},
        Kind kind = Kind::Narration,
        Prepare prepare = {}
    );
    Q_INVOKABLE void request(const QString& text, const QString& endpoint);
    Q_INVOKABLE void requestSoundMaterial(
        const QString& prompt,
        int durationSeconds,
        bool ambience,
        const QString& endpoint,
        const QString& model = QStringLiteral("stable_audio_3_small_sfx")
    );
    Q_INVOKABLE void discard();
    Q_INVOKABLE void stop();
    Q_INVOKABLE void accept(const QString& assembly, bool global);
    Q_INVOKABLE void selectCandidate(const QString& id);
    Q_INVOKABLE void removeSelected();
    QVariantList candidates() const;
    QString selectedCandidateId() const {
        return selected_id_;
    }
    bool running() const {
        return running_;
    }
    bool preparing() const {
        return preparing_;
    }
    bool stopping() const {
        return stopping_;
    }
    bool accepting() const {
        return accepting_;
    }
    QString detailsJson() const;
    QUrl audioUrl() const;
    QString errorText() const {
        return error_;
    }
  signals:
    void stateChanged();
    void accepted(const QString& assetId);

  private:
    void start(const GeneratedAudioRequest& input, const QString& endpoint);
    void beginGeneration(const GeneratedAudioRequest& input, const QString& endpoint);
    Kind kind_;
    Prepare prepare_;
    struct Preparation {
        QString text, endpoint, json;
    };
    std::vector<Preparation> preparations_;
    Generate generate_;
    struct Candidate {
        QString id, details, text;
        std::shared_ptr<GeneratedAudioResult> result;
        std::shared_ptr<QTemporaryDir> directory;
    };
    const Candidate* selected() const;
    QString catalog_, error_, selected_id_;
    std::vector<Candidate> candidates_;
    quint64 generation_ = 0;
    bool running_ = false, accepting_ = false, preparing_ = false, stopping_ = false;
};
