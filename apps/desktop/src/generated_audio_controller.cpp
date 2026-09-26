#include "generated_audio_controller.hpp"
#include "echo-desktop-bridge/src/lib.rs.h"
#include <QFileInfo>
#include <QFutureWatcher>
#include <QRandomGenerator>
#include <QTemporaryDir>
#include <QUuid>
#include <QtConcurrent/QtConcurrentRun>
#include <algorithm>
#include <utility>

namespace {
class RuntimeResult final : public GeneratedAudioResult {
    rust::Box<echo::desktop::GeneratedAudioCandidate> candidate_;

  public:
    explicit RuntimeResult(rust::Box<echo::desktop::GeneratedAudioCandidate> candidate) :
        candidate_(std::move(candidate)) {}
    QString details() const override {
        const auto json = echo::desktop::generated_audio_candidate_details(*candidate_);
        return QString::fromUtf8(json.data(), static_cast<qsizetype>(json.size()));
    }
    QString accept(const QString& catalog, const QString& assembly, bool global) const override {
        const auto id = echo::desktop::accept_generated_audio_candidate(
            catalog.toStdString(),
            *candidate_,
            assembly.toStdString(),
            global
        );
        return QString::fromUtf8(id.data(), static_cast<qsizetype>(id.size()));
    }
};
struct Outcome {
    std::shared_ptr<GeneratedAudioResult> candidate;
    QString details;
};
} // namespace

GeneratedAudioController::GeneratedAudioController(
    QString catalog,
    QObject* parent,
    Generate generate,
    Kind kind,
    Prepare prepare
) :
    QObject(parent), kind_(kind), prepare_(std::move(prepare)), generate_(std::move(generate)),
    catalog_(QFileInfo(catalog).absoluteFilePath()) {
    if (!prepare_)
        prepare_ = [](const QString& prompt, const QString& endpoint) {
            const auto json = echo::desktop::prepare_sound_material_prompt(
                prompt.toStdString(),
                endpoint.toStdString()
            );
            return QString::fromUtf8(json.data(), static_cast<qsizetype>(json.size()));
        };
    if (!generate_)
        generate_ = [kind](
                        const GeneratedAudioRequest& input,
                        const QString& directory,
                        const QString& endpoint
                    ) {
            if (kind == Kind::SoundMaterial) {
                echo::desktop::SoundMaterialRequestWire request;
                request.model = input.model.toStdString();
                request.prompt = input.text.toStdString();
                request.preparation = input.preparation.toStdString();
                request.duration_seconds = static_cast<std::uint32_t>(input.durationSeconds);
                request.seed = input.seed;
                request.ambience = input.ambience;
                return std::make_shared<RuntimeResult>(
                    echo::desktop::generate_sound_material_candidate(
                        request,
                        directory.toStdString(),
                        endpoint.toStdString()
                    )
                );
            }
            return std::make_shared<RuntimeResult>(echo::desktop::generate_narration_candidate(
                input.text.toStdString(),
                directory.toStdString(),
                endpoint.toStdString()
            ));
        };
}
const GeneratedAudioController::Candidate* GeneratedAudioController::selected() const {
    const auto found =
        std::find_if(candidates_.begin(), candidates_.end(), [this](const auto& value) {
            return value.id == selected_id_;
        });
    return found == candidates_.end() ? nullptr : &*found;
}
QString GeneratedAudioController::detailsJson() const {
    const auto* value = selected();
    return value ? value->details : QString{};
}
QUrl GeneratedAudioController::audioUrl() const {
    const auto* value = selected();
    return value ? QUrl::fromLocalFile(value->directory->filePath(
                       kind_ == Kind::SoundMaterial ? QStringLiteral("sound.wav")
                                                    : QStringLiteral("narration.wav")
                   ))
                 : QUrl{};
}
QVariantList GeneratedAudioController::candidates() const {
    QVariantList values;
    for (const auto& value : candidates_)
        values.append(
            QVariantMap{{QStringLiteral("id"), value.id}, {QStringLiteral("text"), value.text}}
        );
    return values;
}
void GeneratedAudioController::selectCandidate(const QString& id) {
    if (accepting_ || id == selected_id_)
        return;
    if (std::none_of(candidates_.begin(), candidates_.end(), [&](const auto& value) {
            return value.id == id;
        }))
        return;
    selected_id_ = id;
    error_.clear();
    emit stateChanged();
}
void GeneratedAudioController::removeSelected() {
    if (running_ || accepting_)
        return;
    std::erase_if(candidates_, [this](const auto& value) { return value.id == selected_id_; });
    selected_id_ = candidates_.empty() ? QString{} : candidates_.back().id;
    error_.clear();
    emit stateChanged();
}
void GeneratedAudioController::stop() {
    if (!running_ || accepting_ || stopping_)
        return;
    ++generation_;
    stopping_ = true;
    error_.clear();
    emit stateChanged();
}
void GeneratedAudioController::discard() {
    if (accepting_)
        return;
    ++generation_;
    stopping_ = running_;
    preparations_.clear();
    candidates_.clear();
    selected_id_.clear();
    error_.clear();
    emit stateChanged();
}
void GeneratedAudioController::request(const QString& text, const QString& endpoint) {
    if (kind_ != Kind::Narration)
        return;
    start({text.trimmed()}, endpoint);
}
void GeneratedAudioController::requestSoundMaterial(
    const QString& prompt,
    int durationSeconds,
    bool ambience,
    const QString& endpoint,
    const QString& model
) {
    if (kind_ != Kind::SoundMaterial)
        return;
    start(
        {prompt.trimmed(),
         durationSeconds,
         QRandomGenerator::global()->generate(),
         ambience,
         model},
        endpoint
    );
}
void GeneratedAudioController::start(const GeneratedAudioRequest& input, const QString& endpoint) {
    if (running_ || accepting_)
        return;
    error_.clear();
    if (candidates_.size() >= 3) {
        error_ = tr("Keep up to three candidates. Remove one before generating another.");
        emit stateChanged();
        return;
    }
    const bool knownModel = input.model == QStringLiteral("stable_audio_3_small_sfx")
                            || input.model == QStringLiteral("stable_audio_3_small_music")
                            || input.model == QStringLiteral("stable_audio_open_small");
    const int maxSeconds = input.model == QStringLiteral("stable_audio_open_small") ? 11 : 30;
    if (input.text.isEmpty() || input.text.toUcs4().size() > 500 || input.text.contains(QChar::Null)
        || (kind_ == Kind::SoundMaterial
            && (!knownModel || input.durationSeconds < 1 || input.durationSeconds > maxSeconds))) {
        error_ = kind_ == Kind::SoundMaterial
                     ? tr("Choose a supported model, enter 1–500 prompt characters, and keep the "
                          "duration within its limit.")
                     : tr("Enter between 1 and 500 characters for the narration.");
        emit stateChanged();
        return;
    }
    running_ = true;
    stopping_ = false;
    if (kind_ != Kind::SoundMaterial) {
        beginGeneration(input, endpoint);
        return;
    }
    const auto normalized = input.text.simplified();
    const auto cached =
        std::find_if(preparations_.begin(), preparations_.end(), [&](const auto& value) {
            return value.text == normalized && value.endpoint == endpoint;
        });
    if (cached != preparations_.end()) {
        auto prepared = input;
        prepared.preparation = cached->json;
        beginGeneration(prepared, endpoint);
        return;
    }
    preparing_ = true;
    emit stateChanged();
    const auto generation = generation_;
    auto* watcher = new QFutureWatcher<QString>(this);
    connect(
        watcher,
        &QFutureWatcher<QString>::finished,
        this,
        [this, watcher, generation, input, endpoint, normalized] {
            const auto preparation = watcher->result();
            watcher->deleteLater();
            preparing_ = false;
            if (generation != generation_ || preparation.isEmpty()) {
                running_ = false;
                stopping_ = false;
                if (generation == generation_)
                    error_ =
                        tr("Could not prepare the sound description locally. Check Infer Runtime "
                           "and Echo’s text access, then try again. No audio was generated.");
                emit stateChanged();
                return;
            }
            if (preparations_.size() >= 4)
                preparations_.erase(preparations_.begin());
            preparations_.push_back({normalized, endpoint, preparation});
            auto prepared = input;
            prepared.preparation = preparation;
            beginGeneration(prepared, endpoint);
        }
    );
    watcher->setFuture(QtConcurrent::run([input, endpoint, prepare = prepare_] {
        try {
            return prepare(input.text, endpoint);
        } catch (const std::exception&) {
            return QString{};
        }
    }));
}
void GeneratedAudioController::beginGeneration(
    const GeneratedAudioRequest& input,
    const QString& endpoint
) {
    auto directory = std::make_shared<QTemporaryDir>();
    if (!directory->isValid()) {
        running_ = false;
        error_ = tr("Could not create a temporary audio file.");
        emit stateChanged();
        return;
    }
    running_ = true;
    const auto generation = generation_;
    emit stateChanged();
    auto* watcher = new QFutureWatcher<Outcome>(this);
    connect(
        watcher,
        &QFutureWatcher<Outcome>::finished,
        this,
        [this, watcher, generation, directory, input] {
            auto outcome = watcher->result();
            watcher->deleteLater();
            running_ = false;
            stopping_ = false;
            if (generation == generation_) {
                if (outcome.candidate) {
                    selected_id_ = QUuid::createUuid().toString(QUuid::WithoutBraces);
                    candidates_.push_back(
                        {selected_id_,
                         outcome.details,
                         input.text,
                         std::move(outcome.candidate),
                         directory}
                    );
                } else
                    error_ = kind_ == Kind::SoundMaterial
                                 ? tr("Sound generation is unavailable. Check Infer Runtime, its "
                                      "sound model and Echo’s generation access.")
                                 : tr("Narration is unavailable. Check Infer Runtime, its speech "
                                      "model and "
                                      "Echo's speech access.");
            }
            emit stateChanged();
        }
    );
    watcher->setFuture(QtConcurrent::run([directory, input, endpoint, generate = generate_] {
        Outcome outcome;
        try {
            outcome.candidate = generate(input, directory->path(), endpoint);
            outcome.details = outcome.candidate->details();
        } catch (const std::exception&) {
            outcome.candidate.reset();
        }
        return outcome;
    }));
}
void GeneratedAudioController::accept(const QString& assembly, bool global) {
    const auto* value = selected();
    if (running_ || accepting_ || !value)
        return;
    accepting_ = true;
    error_.clear();
    emit stateChanged();
    auto* watcher = new QFutureWatcher<QString>(this);
    connect(watcher, &QFutureWatcher<QString>::finished, this, [this, watcher] {
        const auto id = watcher->result();
        watcher->deleteLater();
        accepting_ = false;
        if (id.isEmpty())
            error_ = tr(
                "Could not save this generated sound. The candidate is still available; try again."
            );
        else {
            discard();
            emit accepted(id);
        }
        emit stateChanged();
    });
    watcher->setFuture(
        QtConcurrent::run([catalog = catalog_,
                           candidate = value->result,
                           directory = value->directory,
                           assembly,
                           global] {
            try {
                return candidate->accept(catalog, assembly, global);
            } catch (const std::exception&) {
                return QString{};
            }
        })
    );
}
