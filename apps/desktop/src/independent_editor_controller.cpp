#include "independent_editor_controller.hpp"
#include "echo-desktop-bridge/src/lib.rs.h"
#include "editor_recovery.hpp"
#include "rust/cxx.h"
#include <QCoreApplication>
#include <QDir>
#include <QEventLoop>
#include <QFileInfo>
#include <QFileOpenEvent>
#include <QProcess>
#include <QTimer>
#include <QUuid>
#include <QtConcurrentRun>

namespace {
QString absolute(const QString& path) {
    return QFileInfo(path).absoluteFilePath();
}
} // namespace

IndependentEditorController::IndependentEditorController(QObject* parent) : QObject(parent) {
    const auto args = QCoreApplication::arguments();
    for (int i = 1; i < args.size(); ++i) {
        if (args[i] == QStringLiteral("--edit")) {
            independent_ = true;
        } else if (args[i] == QStringLiteral("--project") && i + 1 < args.size()) {
            independent_ = true;
            projectPath_ = absolute(args[++i]);
        } else if (args[i] == QStringLiteral("--resume-editor") && i + 1 < args.size()) {
            independent_ = true;
            resumePath_ = absolute(args[++i]);
        } else if (independent_) {
            initialFiles_.append(absolute(args[i]));
        } else if (args.size() == 2 && QFileInfo(args[i]).suffix() == QStringLiteral("echo")) {
            independent_ = true;
            projectPath_ = absolute(args[i]);
        }
    }
    QCoreApplication::instance()->installEventFilter(this);
    connect(&recoveryWatcher_, &QFutureWatcher<QVariantList>::finished, this, [this] {
        recoverableSessions_ = recoveryWatcher_.result();
        emit recoveryChanged();
    });
    connect(&watcher_, &QFutureWatcher<Result>::finished, this, [this] {
        const auto result = watcher_.result();
        errorText_ = result.error;
        if (!result.savedPath.isEmpty() && errorText_.isEmpty()) {
            projectPath_ = result.savedPath;
            emit projectSaved();
        }
        if (!result.ids.isEmpty())
            emit audioImported(result.ids);
        emit stateChanged();
    });
}

bool IndependentEditorController::prepare() {
    // Enter the native event loop before selecting a catalog. macOS delivers
    // launch-time file-open events here, not during a processEvents() flush.
    QEventLoop launchEvents;
    QTimer::singleShot(0, &launchEvents, &QEventLoop::quit);
    launchEvents.exec();
    prepared_ = true;
    if (!independent_)
        return true;
    previousDirectory_ = QDir::currentPath();
    const QString home = EditorRecovery::sessionHome();
    sessionHome_ = home;
    if (!QDir().mkpath(home)) {
        errorText_ = tr("The editing workspace could not be created.");
        return false;
    }
    root_ = resumePath_.isEmpty()
                ? QDir(home).filePath(QUuid::createUuid().toString(QUuid::WithoutBraces))
                : resumePath_;
    if (!resumePath_.isEmpty()
        && (!QFileInfo(root_).canonicalFilePath().startsWith(
                QFileInfo(home).canonicalFilePath() + QDir::separator()
            )
            || !QFileInfo::exists(QDir(root_).filePath(QStringLiteral("catalog.sqlite"))))) {
        errorText_ = tr("This recovery session is unavailable.");
        return false;
    }
    try {
        if (!projectPath_.isEmpty())
            echo::desktop::editor_open_project(projectPath_.toStdString(), root_.toStdString());
        else if (!QDir().mkpath(root_)) {
            errorText_ = tr("The editing workspace could not be created.");
            return false;
        }
    } catch (const rust::Error& error) {
        errorText_ = tr("The project could not be opened: %1").arg(QString::fromUtf8(error.what()));
        return false;
    }
    lock_ = std::make_unique<QLockFile>(QDir(root_).filePath(QStringLiteral("session.lock")));
    if (!lock_->tryLock()) {
        errorText_ = tr("This editing session is already open.");
        return false;
    }
    if (!QDir::setCurrent(root_)) {
        errorText_ = tr("The editing workspace could not be opened.");
        return false;
    }
    refreshRecoverableSessions();
    return true;
}

IndependentEditorController::~IndependentEditorController() {
    watcher_.waitForFinished();
    recoveryWatcher_.waitForFinished();
    lock_.reset();
    if (!previousDirectory_.isEmpty())
        QDir::setCurrent(previousDirectory_);
    if (independent_ && cleanExit_ && !root_.isEmpty())
        QDir(root_).removeRecursively();
}

QVariantList IndependentEditorController::recoverableSessions() const {
    return recoverableSessions_;
}

void IndependentEditorController::refreshRecoverableSessions() {
    if (!independent_ || recoveryWatcher_.isRunning())
        return;
    const auto home = sessionHome_;
    const auto current = root_;
    recoveryWatcher_.setFuture(QtConcurrent::run([home, current] {
        return EditorRecovery::list(home, current);
    }));
}

void IndependentEditorController::importAudio(const QList<QUrl>& files) {
    if (!independent_ || busy() || files.isEmpty())
        return;
    errorText_.clear();
    const QString root = root_;
    watcher_.setFuture(QtConcurrent::run([root, files] {
        Result result;
        for (const auto& file : files) {
            if (!file.isLocalFile()) {
                result.error = tr("Choose a local audio file.");
                break;
            }
            try {
                const auto id = echo::desktop::editor_import_audio(
                    root.toStdString(),
                    file.toLocalFile().toStdString()
                );
                result.ids.append(QString::fromUtf8(id.data(), id.size()));
            } catch (const rust::Error& error) {
                result.error =
                    tr("Audio could not be opened: %1").arg(QString::fromUtf8(error.what()));
                break;
            }
        }
        return result;
    }));
    emit stateChanged();
}

void IndependentEditorController::saveProject(const QUrl& destination) {
    if (!independent_ || busy())
        return;
    if (!destination.isLocalFile()) {
        errorText_ = tr("Choose a local project destination.");
        emit stateChanged();
        return;
    }
    QString path = destination.toLocalFile();
    if (!path.endsWith(QStringLiteral(".echo"), Qt::CaseInsensitive))
        path += QStringLiteral(".echo");
    errorText_.clear();
    const QString root = root_;
    watcher_.setFuture(QtConcurrent::run([root, path] {
        Result result;
        try {
            echo::desktop::editor_save_project(root.toStdString(), path.toStdString());
            result.savedPath = path;
        } catch (const rust::Error& error) {
            result.error =
                tr("The project could not be saved: %1").arg(QString::fromUtf8(error.what()));
        }
        return result;
    }));
    emit stateChanged();
}

bool IndependentEditorController::spawn(const QStringList& arguments) {
    const bool started =
        QProcess::startDetached(QCoreApplication::applicationFilePath(), arguments);
    if (!started) {
        errorText_ = tr("The editor window could not be opened.");
        emit stateChanged();
    }
    return started;
}
bool IndependentEditorController::launchEditor(const QList<QUrl>& files) {
    QStringList args{QStringLiteral("--edit")};
    for (const auto& file : files) {
        if (!file.isLocalFile())
            return false;
        args.append(file.toLocalFile());
    }
    return spawn(args);
}
bool IndependentEditorController::launchProject(const QUrl& file) {
    return file.isLocalFile() && spawn({QStringLiteral("--project"), file.toLocalFile()});
}
bool IndependentEditorController::resumeSession(const QString& directory) {
    return spawn({QStringLiteral("--resume-editor"), directory});
}
void IndependentEditorController::finishSession() {
    if (!busy())
        cleanExit_ = true;
}
bool IndependentEditorController::eventFilter(QObject* object, QEvent* event) {
    if (event->type() == QEvent::FileOpen) {
        const QUrl file = static_cast<QFileOpenEvent*>(event)->url();
        if (!prepared_ && file.isLocalFile()) {
            independent_ = true;
            if (file.toLocalFile().endsWith(QStringLiteral(".echo"), Qt::CaseInsensitive))
                projectPath_ = absolute(file.toLocalFile());
            else
                initialFiles_.append(absolute(file.toLocalFile()));
            return true;
        }
        if (file.toLocalFile().endsWith(QStringLiteral(".echo"), Qt::CaseInsensitive))
            launchProject(file);
        else
            launchEditor({file});
        return true;
    }
    return QObject::eventFilter(object, event);
}
