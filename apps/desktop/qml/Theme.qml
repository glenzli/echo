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
        return uiPrefs !== undefined ? uiPrefs.dark : true;
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

    // Editing workbench geometry. Audio parameters remain readable at a
    // stable measure instead of stretching with the window like a timeline.
    // The empty remainder is intentional workspace, matching Shadow's
    // bounded navigator / inspector convention.
    readonly property int editorRailWidth: 210
    readonly property int editorParameterMaxWidth: 820
    readonly property int editorControlTrackWidth: 260
    readonly property int editorSectionColumnWidth: 320
    readonly property int editorPanelGap: 12

    // Base surfaces
    readonly property color window: effectiveDark ? "#0c1114" : "#eef2f4"
    readonly property color chrome: effectiveDark ? "#11171a" : "#fbfcfd"
    readonly property color panel: effectiveDark ? "#151c20" : "#f5f8f9"
    readonly property color panelRaised: effectiveDark ? "#1a2227" : "#ffffff"
    readonly property color panelInset: effectiveDark ? "#10171b" : "#edf2f4"
    readonly property color surfaceSubtle: effectiveDark ? "#202a30" : "#f1f5f6"
    readonly property color surfaceSelected: effectiveDark ? "#26343a" : "#e7f0f2"
    readonly property color parameterPanel: effectiveDark ? "#151c20" : "#f9fbfc"
    readonly property color parameterSection: effectiveDark ? "#1b2429" : "#ffffff"
    readonly property color parameterGraph: effectiveDark ? "#10171b" : "#f4f8f9"
    readonly property color graphGrid: effectiveDark ? "#29363c" : "#dfe8eb"
    readonly property color graphGridStrong: effectiveDark ? "#405158" : "#c5d4d9"
    readonly property color graphFill: effectiveDark ? "#27454d" : "#dbeef1"
    readonly property color waveformSurface: effectiveDark ? "#10191d" : "#e8f0f2"
    readonly property color waveformFill: effectiveDark ? "#8eb3bc" : "#52717a"
    readonly property color waveformPlayed: effectiveDark ? "#b9deda" : "#277d82"
    readonly property color waveformCenter: effectiveDark ? "#435a62" : "#c6d4d8"
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
    readonly property color focusRing: effectiveDark ? "#75acbd" : "#4d899e"

    // Structure and borders
    readonly property color border: effectiveDark ? "#2b373e" : "#dbe3e7"
    readonly property color borderStrong: effectiveDark ? "#43545c" : "#c4d1d6"
    readonly property color separatorStrong: effectiveDark ? "#3d4c53" : "#b8c8ce"
    readonly property color track: effectiveDark ? "#3a4951" : "#ced9de"

    // Text and icons
    readonly property color textPrimary: effectiveDark ? "#f1f4f6" : "#20252a"
    readonly property color textSecondary: effectiveDark ? "#cbd3d9" : "#4f5964"
    readonly property color textMuted: effectiveDark ? "#9ea9b4" : "#626e79"
    readonly property color textDisabled: effectiveDark ? "#74818d" : "#a4acb5"
    readonly property color accent: effectiveDark ? "#69b7c2" : "#147f98"
    readonly property color accentText: effectiveDark ? "#0d1117" : "#ffffff"
    readonly property color accentSurfaceQuiet: effectiveDark ? "#192e34" : "#eef7f8"
    readonly property color accentSurface: effectiveDark ? "#224650" : "#dceff2"
    readonly property color accentBorder: effectiveDark ? "#579aa7" : "#87b7c1"
    readonly property color accentSelectionText: effectiveDark ? "#bce3e8" : "#116f85"

    // Editing surfaces use quiet depth, not desktop-style full-width boxes.
    readonly property color switchOffSurface: effectiveDark ? "#34434a" : "#d3dde1"
    readonly property color switchThumb: effectiveDark ? "#f2f7f8" : "#ffffff"
    readonly property color shadowSoft: effectiveDark ? "#44000000" : "#160f2530"
    readonly property color shadowStrong: effectiveDark ? "#66000000" : "#24132c38"

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
