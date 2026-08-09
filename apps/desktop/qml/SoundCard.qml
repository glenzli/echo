//! A sound's visual surrogate for parallel Library browsing. The card renders
//! real cached waveform evidence plus source and model-derived metadata.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: card

    required property var entry
    required property bool selected
    property var waveformLevels: []

    signal activated()
    signal opened()
    signal affinityRequested(bool liked, int rating)

    readonly property int toneIndex: Math.abs(String(entry.id).charCodeAt(0)
        + String(entry.id).charCodeAt(String(entry.id).length - 1)) % 4
    readonly property color toneStart: ["#173746", "#2c3443", "#304442", "#3d3948"][toneIndex]
    readonly property color toneEnd: ["#668e96", "#8d765b", "#73918a", "#8a7180"][toneIndex]
    readonly property string displayTitle: titleFor(entry)
    readonly property string insightLine: insightFor(entry)

    implicitWidth: 286
    implicitHeight: 206
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
        if (asset.summary.length > 0) {
            return compactText(asset.summary, 34)
        }
        if (asset.sourceTitle.length > 0) {
            return compactText(asset.sourceTitle, 34)
        }
        if (asset.textPreview.length > 0) {
            return compactText(asset.textPreview, 28)
        }
        return fileName(asset.path)
    }

    function insightFor(asset: var) : string {
        const parts = []
        if (asset.eventType.length > 0) {
            parts.push(asset.eventType)
        }
        if (asset.mood.length > 0) {
            parts.push(asset.mood)
        }
        for (let index = 0; index < Math.min(2, asset.keywords.length); ++index) {
            parts.push(String(asset.keywords[index]))
        }
        if (parts.length === 0) {
            parts.push(asset.textPreview.length > 0 ? qsTr("AI-extracted text")
                                                    : asset.codec.toUpperCase())
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
        height: 125
        z: 1
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: card.toneStart }
            GradientStop { position: 1.0; color: card.toneEnd }
        }

        Repeater {
            model: 11
            Rectangle {
                required property int index
                x: index * soundVisual.width / 11
                width: 1
                height: soundVisual.height
                color: "#20ffffff"
            }
        }

        WaveformView {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            anchors.topMargin: 26
            anchors.bottomMargin: 17
            levels: card.waveformLevels
            fillColor: "#d9f4f8"
            progressColor: "#ffffff"
            progress: 0
        }

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.leftMargin: 9
            anchors.topMargin: 8
            width: evidenceText.implicitWidth + 12
            height: 21
            radius: 5
            color: "#660a1820"

            Text {
                id: evidenceText
                anchors.centerIn: parent
                text: card.entry.textPreview.length > 0
                    ? qsTr("✦ AI text") : qsTr("Source metadata")
                color: "#f3fbff"
                font.pixelSize: 9
                font.bold: true
            }
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
        anchors.rightMargin: 8
        anchors.topMargin: 8
        anchors.bottomMargin: 7
        spacing: 3
        z: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Text {
                Layout.fillWidth: true
                text: card.displayTitle
                color: Theme.textPrimary
                font.pixelSize: 12
                font.bold: true
                elide: Text.ElideRight
            }

            Button {
                implicitWidth: 22
                implicitHeight: 22
                padding: 0
                onClicked: card.affinityRequested(!card.entry.liked, card.entry.rating)

                background: Rectangle {
                    radius: 5
                    color: parent.hovered ? Theme.surfaceSubtle : Theme.transparent
                }

                contentItem: Text {
                    text: card.entry.liked ? "♥" : "♡"
                    color: card.entry.liked ? "#dc4b6b" : Theme.textDisabled
                    font.pixelSize: 15
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: card.insightLine
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            elide: Text.ElideRight
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 5

            Text {
                text: card.entry.sourceLocation.length > 0
                    ? card.entry.sourceLocation
                    : card.formatDate(card.entry.recordedAtMillis > 0
                        ? card.entry.recordedAtMillis : card.entry.importedAtMillis)
                color: Theme.textDisabled
                font.pixelSize: 9
                elide: Text.ElideRight
                Layout.maximumWidth: card.width * 0.43
            }

            Item { Layout.fillWidth: true }

            Row {
                spacing: 0

                Repeater {
                    model: 5

                    delegate: Text {
                        required property int index
                        text: index < card.entry.rating ? "★" : "☆"
                        color: index < card.entry.rating ? "#d89a16" : Theme.textDisabled
                        font.pixelSize: 11

                        MouseArea {
                            anchors.fill: parent
                            anchors.margins: -2
                            cursorShape: Qt.PointingHandCursor
                            onClicked: card.affinityRequested(
                                card.entry.liked,
                                card.entry.rating === parent.index + 1 ? 0 : parent.index + 1)
                        }
                    }
                }
            }
        }
    }
}
