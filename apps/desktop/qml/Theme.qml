pragma Singleton

//! Echo theme tokens, mirroring Shadow's Theme contract. `mode` selects
//! System / Light / Dark and is bound by the shell from UiPreferences;
//! `effectiveDark` resolves the actual palette.

import QtQuick

QtObject {
    enum AppearanceMode {
        System,
        Light,
        Dark
    }

    property int mode: Theme.AppearanceMode.System
    readonly property bool effectiveDark: {
        if (mode === Theme.AppearanceMode.Dark) {
            return true
        }
        if (mode === Theme.AppearanceMode.Light) {
            return false
        }
        return uiPrefs !== undefined ? uiPrefs.dark : true
    }

    // Density tokens
    readonly property int compactControlHeight: 30
    readonly property int controlHeight: 34
    readonly property int compactControlRadius: 5
    readonly property int controlRadius: 6
    readonly property int panelRadius: 10
    readonly property int sectionSpacing: 14
    readonly property int panelPadding: 16
    readonly property int fontBody: 12
    readonly property int fontMeta: 10
    readonly property int fontSection: 11

    // Base surfaces
    readonly property color window: effectiveDark ? "#0f1114" : "#f1f3f5"
    readonly property color chrome: effectiveDark ? "#121519" : "#fcfcfd"
    readonly property color panel: effectiveDark ? "#171a1f" : "#f8f9fb"
    readonly property color panelRaised: effectiveDark ? "#1e2329" : "#ffffff"
    readonly property color surfaceSubtle: effectiveDark ? "#20252b" : "#f3f5f7"
    readonly property color surfaceSelected: effectiveDark ? "#252b32" : "#e9eef3"
    readonly property color waveformSurface: effectiveDark ? "#0b0e11" : "#e8ebee"
    readonly property color control: effectiveDark ? "#1d2228" : "#ffffff"
    readonly property color controlQuiet: effectiveDark ? "#20252b" : "#f3f5f7"
    readonly property color controlPressed: effectiveDark ? "#2b323a" : "#e9edf2"

    // Buttons and interactions
    readonly property color buttonSurface: effectiveDark ? "#252d36" : "#ffffff"
    readonly property color buttonHoverSurface: effectiveDark ? "#303b47" : "#f3f6f8"
    readonly property color buttonPressedSurface: effectiveDark ? "#3b4856" : "#e9edf2"
    readonly property color buttonGhostHover: effectiveDark ? "#252f39" : "#f1f3f5"
    readonly property color buttonGhostPressed: effectiveDark ? "#303c48" : "#e7ebef"
    readonly property color buttonBorder: effectiveDark ? "#53616e" : "#d1d8e0"
    readonly property color focusRing: effectiveDark ? "#76a6d4" : "#5a91cb"

    // Structure and borders
    readonly property color border: effectiveDark ? "#323b45" : "#d9dee5"
    readonly property color borderStrong: effectiveDark ? "#485460" : "#c8d0d8"
    readonly property color separatorStrong: effectiveDark ? "#404852" : "#bbc4cd"
    readonly property color track: effectiveDark ? "#3b4652" : "#d0d6dd"

    // Text and icons
    readonly property color textPrimary: effectiveDark ? "#f1f4f6" : "#20252a"
    readonly property color textSecondary: effectiveDark ? "#cbd3d9" : "#4f5964"
    readonly property color textDisabled: effectiveDark ? "#74818d" : "#a4acb5"
    readonly property color accent: effectiveDark ? "#70a3d6" : "#3574b9"
    readonly property color accentText: effectiveDark ? "#0d1117" : "#ffffff"
    readonly property color accentSurfaceQuiet: effectiveDark ? "#1b2a39" : "#f4f8fc"
    readonly property color accentSurface: effectiveDark ? "#223d58" : "#eaf2fa"
    readonly property color accentSelectionText: effectiveDark ? "#b9d6ee" : "#285f9c"

    // Status colors shared with Shadow.
    readonly property color warningSurface: effectiveDark ? "#3b3324" : "#f8ecd2"
    readonly property color warningText: effectiveDark ? "#a99268" : "#76520d"

    readonly property color transparent: "#00000000"
}
