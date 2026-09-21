//! Independent editor process entry and asynchronous project file operations.
//! Recovery discovery and validation isolation live in editor_recovery.hpp.
//! Saved-project history and the start-page projection live in editor_recent_projects.hpp.
#pragma once
#include <QFutureWatcher>
#include <QLockFile>
#include <QObject>
#include <QUrl>
#include <QVariantList>
#include <memory>

class IndependentEditorController final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool independent READ independent CONSTANT)
    Q_PROPERTY(bool busy READ busy NOTIFY stateChanged)
    Q_PROPERTY(QString projectPath READ projectPath NOTIFY stateChanged)
    Q_PROPERTY(QUrl projectUrl READ projectUrl NOTIFY stateChanged)
    Q_PROPERTY(QString errorText READ errorText NOTIFY stateChanged)
    Q_PROPERTY(QVariantList recentProjects READ recentProjects NOTIFY recentProjectsChanged)
  public:
    explicit IndependentEditorController(QObject* parent = nullptr);
    ~IndependentEditorController() override;
    bool prepare();
    bool independent() const {
        return independent_;
    }
    bool busy() const {
        return watcher_.isRunning();
    }
    QString projectPath() const {
        return projectPath_;
    }
    QUrl projectUrl() const {
        return QUrl::fromLocalFile(projectPath_);
    }
    QString errorText() const {
        return errorText_;
    }
    QVariantList recentProjects() const;
    QStringList initialFiles() const {
        return initialFiles_;
    }
    Q_INVOKABLE void importAudio(const QList<QUrl>& files);
    Q_INVOKABLE void saveProject(const QUrl& destination);
    Q_INVOKABLE bool launchEditor(const QList<QUrl>& files = {});
    Q_INVOKABLE bool launchProject(const QUrl& file);
    Q_INVOKABLE bool resumeSession(const QString& directory);
    Q_INVOKABLE bool openRecentProject(const QString& path, bool recovery);
    Q_INVOKABLE void refreshRecentProjects();
    Q_INVOKABLE void finishSession();
  signals:
    void stateChanged();
    void recentProjectsChanged();
    void audioImported(const QStringList& ids);
    void projectSaved();

  protected:
    bool eventFilter(QObject* object, QEvent* event) override;

  private:
    struct Result {
        QString error;
        QStringList ids;
        QString savedPath;
    };
    bool spawn(const QStringList& arguments);
    bool prepared_ = false;
    bool independent_ = false;
    bool cleanExit_ = false;
    QString root_, sessionHome_, projectPath_, resumePath_, errorText_, previousDirectory_;
    QStringList initialFiles_;
    std::unique_ptr<QLockFile> lock_;
    QFutureWatcher<Result> watcher_;
    QFutureWatcher<QVariantList> recentWatcher_;
    QVariantList recentProjects_;
};
