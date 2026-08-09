//! A sound's information-first visual surrogate for parallel Library browsing.
//! Density changes the evidence disclosed by the card while a compact, real,
//! width-selected waveform remains a secondary texture and playback landmark.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: card

    required property var entry
    required property bool selected
    required property string density
    required property int displayHeight
    property var waveformLevels: []

    signal activated()
    signal opened()

    readonly property bool overview: density === "overview"
    readonly property bool rich: density === "rich"
    readonly property var tonePair: toneFor(entry.recordedAtMillis,
                                            Theme.effectiveDark)
    readonly property color toneStart: tonePair.start
    readonly property color toneEnd: tonePair.end
    readonly property string displayTitle: titleFor(entry)
    readonly property string keywordLine: keywordLineFor(entry)
    readonly property string sourceLine: sourceFor(entry)
    readonly property string evidenceLine: evidenceLineFor(entry)
    readonly property int waveformHeight: Math.max(62,
        Math.round(displayHeight * 0.35))

    implicitWidth: 286
    implicitHeight: displayHeight
    radius: 10
    color: Theme.panelRaised
    border.width: selected ? 2 : 1
    border.color: selected ? Theme.accent : Theme.border
    clip: true

    Accessible.role: Accessible.ListItem
    Accessible.name: displayTitle
    Accessible.selected: selected

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function compactText(text: string, maximum: int) : string {
        const normalized = String(text || "").replace(/\s+/g, " ").trim()
        if (normalized.length <= maximum) {
            return normalized
        }
        return normalized.substring(0, maximum).trim() + "…"
    }

    function titleFor(asset: var) : string {
        const maximum = rich ? 96 : overview ? 56 : 76
        if (asset.soundCaption.length > 0) {
            return compactText(asset.soundCaption, maximum)
        }
        if (asset.sourceTitle.length > 0) {
            return compactText(asset.sourceTitle, maximum)
        }
        return fileName(asset.path)
    }

    function keywordLineFor(asset: var) : string {
        const keywords = []
        const keywordLimit = rich ? 3 : 2
        for (let index = 0; index < Math.min(keywordLimit, asset.keywords.length); ++index) {
            keywords.push(String(asset.keywords[index]))
        }
        return keywords.join(" · ")
    }

    function sourceFor(asset: var) : string {
        const parts = []
        if (asset.sourceLocation.length > 0) {
            parts.push(asset.sourceLocation)
        }
        if (asset.codec.length > 0) {
            parts.push(asset.codec.toUpperCase())
        }
        if (asset.sampleRate > 0) {
            parts.push((asset.sampleRate / 1000).toFixed(
                asset.sampleRate % 1000 === 0 ? 0 : 1) + " kHz")
        }
        return parts.join(" · ")
    }

    function evidenceLineFor(asset: var) : string {
        const parts = []
        if (asset.eventType.length > 0) {
            parts.push(asset.eventType)
        }
        if (sourceLine.length > 0) {
            parts.push(sourceLine)
        }
        return parts.join(" · ")
    }

    function formatDuration(millis: double) : string {
        if (!millis || millis <= 0) {
            return "—"
        }
        const totalSeconds = Math.floor(millis / 1000)
        const hours = Math.floor(totalSeconds / 3600)
        const minutes = Math.floor((totalSeconds % 3600) / 60)
        const seconds = totalSeconds % 60
        if (hours > 0) {
            return hours + ":" + String(minutes).padStart(2, "0") + ":"
                + String(seconds).padStart(2, "0")
        }
        return minutes + ":" + String(seconds).padStart(2, "0")
    }

    function formatDate(millis: double) : string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown date")
        }
        return new Date(millis).toLocaleDateString(Qt.locale(), Locale.ShortFormat)
    }

    function toneFor(recordedAtMillis: double, dark: bool) : var {
        const palettes = dark ? {
            dawn: { start: "#293336", end: "#3c4140" },
            day: { start: "#26343a", end: "#35464a" },
            dusk: { start: "#332f37", end: "#493b41" },
            night: { start: "#202832", end: "#2d3742" },
            unknown: { start: "#292e33", end: "#383f45" }
        } : {
            dawn: { start: "#e9eeed", end: "#f2eee8" },
            day: { start: "#e5ecee", end: "#eff3f3" },
            dusk: { start: "#eee8ec", end: "#f3ece9" },
            night: { start: "#e4e9ef", end: "#edf0f4" },
            unknown: { start: "#e9ecef", end: "#f1f3f5" }
        }
        if (!recordedAtMillis || recordedAtMillis <= 0) {
            return palettes.unknown
        }

        const hour = new Date(recordedAtMillis).getHours()
        if (hour >= 5 && hour < 9) {
            return palettes.dawn
        }
        if (hour >= 9 && hour < 17) {
            return palettes.day
        }
        if (hour >= 17 && hour < 21) {
            return palettes.dusk
        }
        return palettes.night
    }

    function loadWaveform() : void {
        if (entry === null || !entry.id || entry.pathStatus === "missing") {
            waveformLevels = []
            return
        }
        try {
            waveformLevels = backend.waveformForAsset(entry.id)
        } catch (error) {
            waveformLevels = []
        }
    }

    Component.onCompleted: Qt.callLater(loadWaveform)

    HoverHandler { id: cardHover }

    Rectangle {
        anchors.fill: parent
        visible: cardHover.hovered && !card.selected
        color: Theme.accentSurfaceQuiet
        opacity: 0.22
        z: 5
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.PointingHandCursor
        z: 6
        onClicked: card.activated()
        onDoubleClicked: card.opened()
    }

    Rectangle {
        id: soundVisual

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: card.waveformHeight
        z: 1
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: card.toneStart }
            GradientStop { position: 1.0; color: card.toneEnd }
        }

        Repeater {
            model: card.overview ? 5 : card.rich ? 17 : 11

            Rectangle {
                required property int index
                x: index * soundVisual.width
                    / (card.overview ? 5 : card.rich ? 17 : 11)
                width: 1
                height: soundVisual.height
                color: Theme.effectiveDark ? "#20ffffff" : "#14000000"
            }
        }

        WaveformView {
            anchors.fill: parent
            anchors.leftMargin: card.rich ? 14 : 10
            anchors.rightMargin: card.rich ? 14 : 10
            anchors.topMargin: card.overview ? 8 : 10
            anchors.bottomMargin: card.overview ? 11 : 13
            levels: card.waveformLevels
            fillColor: Theme.effectiveDark ? "#d9f4f8" : "#52646b"
            progressColor: Theme.effectiveDark ? "#ffffff" : "#2f424a"
            progress: 0
        }

        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: 8
            anchors.bottomMargin: 6
            width: durationText.implicitWidth + 10
            height: 18
            radius: 5
            color: Theme.effectiveDark ? "#66081319" : "#ccffffff"

            Text {
                id: durationText
                anchors.centerIn: parent
                text: card.formatDuration(card.entry.durationMillis)
                color: Theme.effectiveDark ? "#f4f8fa" : "#324148"
                font.pixelSize: 9
            }
        }

        Text {
            anchors.centerIn: parent
            visible: card.waveformLevels.length === 0
            text: card.entry.pathStatus === "missing" ? qsTr("Original missing")
                                                      : qsTr("Preparing waveform…")
            color: Theme.effectiveDark ? "#d0e0e5" : "#52636a"
            font.pixelSize: Theme.fontMeta
        }
    }

    ColumnLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: soundVisual.bottom
        anchors.bottom: parent.bottom
        anchors.leftMargin: 10
        anchors.rightMargin: 9
        anchors.topMargin: card.overview ? 6 : 8
        anchors.bottomMargin: card.overview ? 5 : 7
        spacing: card.rich ? 4 : 3
        z: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                text: card.displayTitle
                color: Theme.textPrimary
                font.pixelSize: card.rich ? 13 : 12
                font.bold: true
                maximumLineCount: 2
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
            }

            Text {
                Layout.alignment: Qt.AlignTop
                visible: card.entry.liked
                text: "♥"
                color: "#dc4b6b"
                font.pixelSize: 14
            }

            Text {
                Layout.alignment: Qt.AlignTop
                visible: card.entry.rating > 0
                text: "★ " + card.entry.rating
                color: "#d89a16"
                font.pixelSize: 10
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: card.entry.mood.length > 0 || card.keywordLine.length > 0
            spacing: 6

            Rectangle {
                Layout.preferredWidth: Math.min(card.width * 0.35,
                                                moodText.implicitWidth + 14)
                Layout.preferredHeight: 21
                radius: 6
                color: Theme.effectiveDark ? "#4a3345" : "#f5e3ef"
                visible: card.entry.mood.length > 0

                Accessible.role: Accessible.StaticText
                Accessible.name: qsTr("Mood: %1").arg(card.entry.mood)

                Text {
                    id: moodText
                    anchors.fill: parent
                    anchors.leftMargin: 7
                    anchors.rightMargin: 7
                    text: card.entry.mood
                    color: Theme.effectiveDark ? "#f4ccdf" : "#8f3f69"
                    font.pixelSize: 9
                    font.bold: true
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }

            Text {
                Layout.fillWidth: true
                visible: card.keywordLine.length > 0
                text: card.keywordLine
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        Text {
            Layout.fillWidth: true
            visible: !card.overview && card.entry.textPreview.length > 0
            text: card.compactText(card.entry.textPreview, card.rich ? 150 : 90)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            maximumLineCount: card.rich ? 3 : 2
            wrapMode: Text.WordWrap
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            visible: !card.overview && card.evidenceLine.length > 0
            text: card.evidenceLine
            color: Theme.textDisabled
            font.pixelSize: 9
            elide: Text.ElideRight
        }

        Item {
            Layout.fillHeight: true
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 5

            Text {
                Layout.fillWidth: true
                text: card.entry.sourceLocation.length > 0
                    ? card.entry.sourceLocation
                    : card.formatDate(card.entry.recordedAtMillis > 0
                        ? card.entry.recordedAtMillis : card.entry.importedAtMillis)
                color: Theme.textDisabled
                font.pixelSize: 9
                elide: Text.ElideRight
            }

            Text {
                visible: card.rich && card.entry.sourceLocation.length > 0
                text: card.formatDate(card.entry.recordedAtMillis > 0
                    ? card.entry.recordedAtMillis : card.entry.importedAtMillis)
                color: Theme.textDisabled
                font.pixelSize: 9
            }
        }
    }
}
