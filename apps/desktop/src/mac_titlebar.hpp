//! macOS title bar integration: positions the native traffic-light buttons at
//! `traffic_light_center_y` points below the window top so they share the
//! toolbar row with the actions. Declared on all platforms; the
//! implementation is macOS-only.

#pragma once

#include <QWindow>

void installMacTitleBarAlignment(QWindow* window, int traffic_light_center_y);
