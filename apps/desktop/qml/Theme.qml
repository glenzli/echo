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
            return true;
        }
        if (mode === Theme.AppearanceMode.Light) {
            return false;
        }
        return typeof uiPrefs !== "undefined" ? uiPrefs.dark : true;
    }

    // Density tokens
    readonly property int compactControlHeight: 30
    readonly property int controlHeight: 34
    readonly property int compactControlRadius: 6
    readonly property int controlRadius: 8
    readonly property int panelRadius: 12
    readonly property int cardRadius: 13
    readonly property int spaceXs: 4
    readonly property int spaceSm: 8
    readonly property int spaceMd: 12
    readonly property int spaceLg: 16
    readonly property int sectionSpacing: 14
    readonly property int panelPadding: 16
    readonly property int fontBody: 12
    readonly property int fontMeta: 10
    readonly property int fontSection: 11
    readonly property var assemblyTrackColors: ["#498eba", "#759b71", "#b49360", "#ad799e", "#6c9ea0", "#af7d63", "#8286ba", "#979561"]

    // Editing workbench geometry. Audio parameters remain readable at a
    // stable measure instead of stretching with the window like a timeline.
    // The empty remainder is intentional workspace, matching Shadow's
    // bounded navigator / inspector convention.
    readonly property int editorRailWidth: 210
    readonly property int editorParameterMaxWidth: 820
    readonly property int editorControlTrackWidth: 260
    readonly property int editorSectionColumnWidth: 320
    readonly property int editorPanelGap: 12
    readonly property int editorPanelHeaderHeight: 38

    // Base surfaces
    readonly property color window: effectiveDark ? "#101214" : "#f1f3f5"
    readonly property color chrome: effectiveDark ? "#141619" : "#fcfcfd"
    readonly property color panel: effectiveDark ? "#181a1e" : "#f7f8fa"
    readonly property color panelRaised: effectiveDark ? "#1e2125" : "#ffffff"
    readonly property color panelInset: effectiveDark ? "#14171a" : "#eef0f2"
    readonly property color surfaceSubtle: effectiveDark ? "#22262b" : "#f1f3f5"
    readonly property color surfaceSelected: effectiveDark ? "#272d34" : "#e9eef3"
    readonly property color parameterPanel: effectiveDark ? "#181a1e" : "#f8f9fa"
    readonly property color parameterSection: effectiveDark ? "#202328" : "#ffffff"
    readonly property color parameterGraph: effectiveDark ? "#14171a" : "#f4f5f7"
    readonly property color graphGrid: effectiveDark ? "#2b3036" : "#e1e4e7"
    readonly property color graphGridStrong: effectiveDark ? "#424951" : "#cbd1d6"
    readonly property color graphFill: effectiveDark ? "#22374c" : "#dfeaf5"
    readonly property color waveformSurface: effectiveDark ? "#151e27" : "#f2f6f9"
    readonly property color waveformFill: effectiveDark ? "#a2c6db" : "#456b84"
    readonly property color waveformPlayed: effectiveDark ? "#72cbd3" : "#176877"
    readonly property color waveformCenter: effectiveDark ? "#354959" : "#ccdbe5"
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
    readonly property color border: effectiveDark ? "#30353b" : "#d9dee5"
    readonly property color borderStrong: effectiveDark ? "#485059" : "#c8d0d8"
    readonly property color separatorStrong: effectiveDark ? "#40474f" : "#bbc4cd"
    readonly property color track: effectiveDark ? "#3b424a" : "#d0d6dd"

    // Text and icons
    readonly property color textPrimary: effectiveDark ? "#f1f4f6" : "#20252a"
    readonly property color textSecondary: effectiveDark ? "#cbd3d9" : "#4f5964"
    readonly property color textMuted: effectiveDark ? "#9ea9b4" : "#626e79"
    readonly property color textDisabled: effectiveDark ? "#74818d" : "#a4acb5"
    readonly property color accent: effectiveDark ? "#70a3d6" : "#3574b9"
    readonly property color accentText: effectiveDark ? "#0d1117" : "#ffffff"
    readonly property color accentSurfaceQuiet: effectiveDark ? "#1b2a39" : "#f4f8fc"
    readonly property color accentSurface: effectiveDark ? "#223d58" : "#eaf2fa"
    readonly property color accentBorder: effectiveDark ? "#5e95c4" : "#91b4d8"
    readonly property color accentSelectionText: effectiveDark ? "#b9d6ee" : "#285f9c"

    // Editing surfaces use quiet depth, not desktop-style full-width boxes.
    readonly property color switchOffSurface: effectiveDark ? "#2e343b" : "#dfe4e8"
    readonly property color switchThumb: effectiveDark ? "#f2f7f8" : "#ffffff"
    readonly property color shadowSoft: effectiveDark ? "#44000000" : "#1618202a"
    readonly property color shadowStrong: effectiveDark ? "#66000000" : "#24151b24"

    // Semantic chips are evidence signals, not navigation controls. Keep them
    // quieter than the accent and stable between the wall and inspector.
    readonly property color eventSurface: effectiveDark ? "#29343a" : "#e9eff1"
    readonly property color eventText: effectiveDark ? "#c1d0d5" : "#52676f"
    readonly property color languageSurface: effectiveDark ? "#302f3f" : "#efedf5"
    readonly property color languageText: effectiveDark ? "#cec9e1" : "#655d7a"

    // Curation colors remain distinct from navigation blue. They are shared
    // by cards, the inspector and floating/bottom toolbars.
    readonly property color likeAccent: effectiveDark ? "#e58a91" : "#cd5b65"
    readonly property color likeSurface: effectiveDark ? "#3e292f" : "#f9e7e9"
    readonly property color ratingAccent: effectiveDark ? "#e3bd68" : "#b78120"
    readonly property color ratingSurface: effectiveDark ? "#3a3222" : "#fbf2df"

    // Status colors shared with Shadow.
    readonly property color warningSurface: effectiveDark ? "#3b3324" : "#f8ecd2"
    readonly property color warningText: effectiveDark ? "#a99268" : "#76520d"

    function moodFamily(value) {
        const mood = String(value || "").toLocaleLowerCase();
        if (mood.includes("calm") || mood.includes("quiet") || mood.includes("peace") || mood.includes("平静") || mood.includes("安静") || mood.includes("放松"))
            return "calm";
        if (mood.includes("happy") || mood.includes("joy") || mood.includes("快乐") || mood.includes("愉快") || mood.includes("开心"))
            return "joy";
        if (mood.includes("sad") || mood.includes("melanch") || mood.includes("悲伤") || mood.includes("忧郁"))
            return "sad";
        if (mood.includes("tense") || mood.includes("angry") || mood.includes("fear") || mood.includes("紧张") || mood.includes("愤怒") || mood.includes("害怕"))
            return "tense";
        return "neutral";
    }

    function moodSurface(value) {
        switch (moodFamily(value)) {
        case "calm":
            return effectiveDark ? "#213b39" : "#e3f2ef";
        case "joy":
            return effectiveDark ? "#40351f" : "#fbf0d7";
        case "sad":
            return effectiveDark ? "#263648" : "#e7eef8";
        case "tense":
            return effectiveDark ? "#462c2f" : "#f8e5e6";
        default:
            return effectiveDark ? "#3b3044" : "#f1e8f6";
        }
    }

    function moodText(value) {
        switch (moodFamily(value)) {
        case "calm":
            return effectiveDark ? "#a9d9d2" : "#31786f";
        case "joy":
            return effectiveDark ? "#e8cb87" : "#8a6419";
        case "sad":
            return effectiveDark ? "#b9cee8" : "#486d9c";
        case "tense":
            return effectiveDark ? "#edb7bb" : "#9a4c52";
        default:
            return effectiveDark ? "#d8c0e7" : "#75508c";
        }
    }

    readonly property color transparent: "#00000000"
}
