//! macOS title bar integration: positions the native traffic-light buttons at
//! the vertical center of the QML title toolbar so they share one row with
//! its actions. Declared on all platforms; the implementation is macOS-only.

#pragma once

#include <QWindow>

void installMacTitleBarAlignment(QWindow* window, int title_bar_height);
