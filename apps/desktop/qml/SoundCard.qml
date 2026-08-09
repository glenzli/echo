//! A sound's visual surrogate for parallel Library browsing. Density changes
//! the evidence disclosed by the card while the full recording duration always
//! remains represented by a real, width-selected waveform pyramid level.

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
    readonly property int toneIndex: Math.abs(String(entry.id).charCodeAt(0)
        + String(entry.id).charCodeAt(String(entry.id).length - 1)) % 4
    readonly property color toneStart: ["#173746", "#2c3443", "#304442", "#3d3948"][toneIndex]
    readonly property color toneEnd: ["#668e96", "#8d765b", "#73918a", "#8a7180"][toneIndex]
    readonly property string displayTitle: titleFor(entry)
    readonly property string keywordLine: keywordLineFor(entry)
    readonly property string sourceLine: sourceFor(entry)

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
        const maximum = rich ? 58 : overview ? 28 : 34
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
        height: card.overview ? 98 : card.rich ? 158 : 125
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
                color: "#20ffffff"
            }
        }

        WaveformView {
            anchors.fill: parent
            anchors.leftMargin: card.rich ? 14 : 10
            anchors.rightMargin: card.rich ? 14 : 10
            anchors.topMargin: 14
            anchors.bottomMargin: 17
            levels: card.waveformLevels
            fillColor: "#d9f4f8"
            progressColor: "#ffffff"
            progress: 0
        }

        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: 8
            anchors.bottomMargin: 7
            width: durationText.implicitWidth + 10
            height: 20
            radius: 5
            color: "#66081319"

            Text {
                id: durationText
                anchors.centerIn: parent
                text: card.formatDuration(card.entry.durationMillis)
                color: "#f4f8fa"
                font.pixelSize: 9
            }
        }

        Text {
            anchors.centerIn: parent
            visible: card.waveformLevels.length === 0
            text: card.entry.pathStatus === "missing" ? qsTr("Original missing")
                                                      : qsTr("Preparing waveform…")
            color: "#d0e0e5"
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
                text: card.displayTitle
                color: Theme.textPrimary
                font.pixelSize: card.rich ? 13 : 12
                font.bold: true
                elide: Text.ElideRight
            }

            Text {
                visible: card.entry.liked
                text: "♥"
                color: "#dc4b6b"
                font.pixelSize: 14
            }

            Text {
                visible: card.entry.rating > 0
                text: "★ " + card.entry.rating
                color: "#d89a16"
                font.pixelSize: 10
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !card.overview
                && (card.entry.mood.length > 0 || card.keywordLine.length > 0)
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
            Layout.fillHeight: card.rich
            visible: card.rich && card.entry.textPreview.length > 0
            text: card.compactText(card.entry.textPreview, 150)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            maximumLineCount: 2
            wrapMode: Text.WordWrap
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            visible: card.rich && card.sourceLine.length > 0
            text: card.sourceLine
            color: Theme.textDisabled
            font.pixelSize: 9
            elide: Text.ElideRight
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !card.overview
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
