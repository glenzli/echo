//! Bounded recovery discovery and explicit isolation of validation sessions.
#pragma once
#include <QString>
#include <QVariantList>

namespace EditorRecovery {
QString sessionHome();
QVariantList list(const QString& home, const QString& current);
} // namespace EditorRecovery
