pragma Singleton

//! Echo theme tokens. One source of design values shared by every Echo
//! component, mirroring Shadow's Theme contract.

import QtQuick

QtObject {
    readonly property color background: "#121417"
    readonly property color surface: "#1B1F24"
    readonly property color surfaceRaised: "#242A31"
    readonly property color textPrimary: "#E8EAED"
    readonly property color textSecondary: "#9AA3AC"
    readonly property color accent: "#4E7CFF"
    readonly property color accentText: "#FFFFFF"
    readonly property color divider: "#2A3138"
}
