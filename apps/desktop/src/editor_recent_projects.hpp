//! Recent saved projects and recoverable edits for the standalone start page.
//! The small path index is local UI history, never a Library catalog.
#pragma once
#include <QString>
#include <QVariantList>

namespace EditorRecentProjects {
bool remember(const QString& home, const QString& project);
QVariantList list(const QString& home, const QString& currentSession);
} // namespace EditorRecentProjects
