#include "generated_narration_controller.hpp"
#include "echo-desktop-bridge/src/lib.rs.h"
#include <QFileInfo>
#include <QFutureWatcher>
#include <QTemporaryDir>
#include <QtConcurrent/QtConcurrentRun>
#include <utility>

namespace {
class RuntimeResult final : public NarrationResult {
    rust::Box<echo::desktop::NarrationCandidate> candidate_;

  public:
    explicit RuntimeResult(rust::Box<echo::desktop::NarrationCandidate> candidate) :
        candidate_(std::move(candidate)) {}
    QString details() const override {
        const auto json = echo::desktop::narration_candidate_details(*candidate_);
        return QString::fromUtf8(json.data(), static_cast<qsizetype>(json.size()));
    }
    QString accept(const QString& catalog, const QString& assembly, bool global) const override {
        const auto id = echo::desktop::accept_narration_candidate(
            catalog.toStdString(),
            *candidate_,
            assembly.toStdString(),
            global
        );
        return QString::fromUtf8(id.data(), static_cast<qsizetype>(id.size()));
    }
};
struct Outcome {
    std::shared_ptr<NarrationResult> candidate;
    QString details;
};
} // namespace

GeneratedNarrationController::GeneratedNarrationController(
    QString catalog,
    QObject* parent,
    Generate generate
) :
    QObject(parent), generate_(std::move(generate)),
    catalog_(QFileInfo(catalog).absoluteFilePath()) {
    if (!generate_)
        generate_ = [](const QString& text, const QString& directory, const QString& endpoint) {
            return std::make_shared<RuntimeResult>(echo::desktop::generate_narration_candidate(
                text.toStdString(),
                directory.toStdString(),
                endpoint.toStdString()
            ));
        };
}
QUrl GeneratedNarrationController::audioUrl() const {
    return candidate_ && directory_
               ? QUrl::fromLocalFile(directory_->filePath(QStringLiteral("narration.wav")))
               : QUrl{};
}
void GeneratedNarrationController::discard() {
    if (accepting_)
        return;
    ++generation_;
    candidate_.reset();
    directory_.reset();
    details_.clear();
    error_.clear();
    emit stateChanged();
}
void GeneratedNarrationController::request(const QString& text, const QString& endpoint) {
    if (running_ || accepting_)
        return;
    discard();
    const auto input = text.trimmed();
    if (input.isEmpty() || input.toUcs4().size() > 500 || input.contains(QChar::Null)) {
        error_ = tr("Enter between 1 and 500 characters for the narration.");
        emit stateChanged();
        return;
    }
    auto directory = std::make_shared<QTemporaryDir>();
    if (!directory->isValid()) {
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
        [this, watcher, generation, directory] {
            auto outcome = watcher->result();
            watcher->deleteLater();
            running_ = false;
            if (generation == generation_) {
                candidate_ = std::move(outcome.candidate);
                if (candidate_) {
                    directory_ = directory;
                    details_ = outcome.details;
                } else
                    error_ =
                        tr("Narration is unavailable. Check Infer Runtime, its speech model and "
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
void GeneratedNarrationController::accept(const QString& assembly, bool global) {
    if (running_ || accepting_ || !candidate_)
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
            error_ =
                tr("Could not save this narration. The candidate is still available; try again.");
        else {
            discard();
            emit accepted(id);
        }
        emit stateChanged();
    });
    watcher->setFuture(
        QtConcurrent::run(
            [catalog = catalog_, candidate = candidate_, directory = directory_, assembly, global] {
                try {
                    return candidate->accept(catalog, assembly, global);
                } catch (const std::exception&) {
                    return QString{};
                }
            }
        )
    );
}
