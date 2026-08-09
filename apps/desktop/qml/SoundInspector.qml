//! Selected-sound quick inspector. It owns lightweight audition, text and
//! metadata presentation; the complete playback workspace is opened explicitly.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: inspector

    required property var asset
    required property var jobStats
    property var waveformLevels: []
    property var textRecords: []
    property string loadedPath: ""

    readonly property bool hasAsset: asset !== null && asset !== undefined

    signal affinityRequested(var asset, bool liked, int rating)

    color: Theme.panelRaised

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function compactText(text: string, maximum: int) : string {
        const normalized = String(text || "").replace(/\s+/g, " ").trim()
        return normalized.length <= maximum ? normalized
            : normalized.substring(0, maximum).trim() + "…"
    }

    function titleFor(selected: var) : string {
        if (!selected) {
            return ""
        }
        if (selected.summary.length > 0) {
            return compactText(selected.summary, 68)
        }
        if (selected.sourceTitle.length > 0) {
            return compactText(selected.sourceTitle, 68)
        }
        if (selected.textPreview.length > 0) {
            return compactText(selected.textPreview, 54)
        }
        return fileName(selected.path)
    }

    function formatDuration(millis: double) : string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown length")
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
            return qsTr("Unknown")
        }
        return new Date(millis).toLocaleString(Qt.locale(), Locale.ShortFormat)
    }

    function refresh() : void {
        waveformLevels = []
        textRecords = []
        if (!asset || !asset.id) {
            return
        }
        if (asset.pathStatus !== "missing") {
            waveformLevels = backend.waveformForAsset(asset.id)
        }
        textRecords = backend.transcriptsForAsset(asset.id)
    }

    function playSelected() : void {
        if (!asset || asset.pathStatus === "missing") {
            return
        }
        if (loadedPath !== asset.path) {
            player.play(asset.path)
            loadedPath = asset.path
        } else {
            player.togglePause()
        }
    }

    function keywordTags() : var {
        const values = []
        if (!asset) {
            return values
        }
        for (let index = 0; index < asset.keywords.length; ++index) {
            values.push(String(asset.keywords[index]))
        }
        return values
    }

    onAssetChanged: Qt.callLater(refresh)

    Connections {
        target: backend
        function onAssetsChanged() : void { inspector.refresh() }
    }

    Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        width: 1
        color: Theme.border
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 50, 270)
        spacing: 9
        visible: !inspector.hasAsset

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a sound")
            color: Theme.textPrimary
            font.pixelSize: 16
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Its waveform, text, source metadata, and AI evidence will appear here.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: inspector.hasAsset

        ScrollView {
            id: detailScroll

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: detailScroll.availableWidth
                spacing: 11

                Item { Layout.preferredHeight: 3 }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 13
                    spacing: 8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset
                                ? inspector.titleFor(inspector.asset) : ""
                            color: Theme.textPrimary
                            font.pixelSize: 18
                            font.bold: true
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset
                                ? inspector.fileName(inspector.asset.path) + " · "
                                    + inspector.formatDuration(inspector.asset.durationMillis)
                                : ""
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideMiddle
                        }
                    }

                    Button {
                        implicitWidth: 28
                        implicitHeight: 28
                        padding: 0
                        onClicked: inspector.affinityRequested(
                            inspector.asset, !inspector.asset.liked, inspector.asset.rating)

                        background: Rectangle {
                            radius: 7
                            color: parent.hovered ? Theme.surfaceSubtle : Theme.transparent
                        }

                        contentItem: Text {
                            text: inspector.hasAsset && inspector.asset.liked ? "♥" : "♡"
                            color: inspector.hasAsset && inspector.asset.liked
                                ? "#dc4b6b" : Theme.textDisabled
                            font.pixelSize: 18
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    spacing: 1

                    Text {
                        text: qsTr("Rating")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }

                    Item { Layout.fillWidth: true }

                    Repeater {
                        model: 5

                        delegate: Text {
                            required property int index
                            text: inspector.hasAsset && index < inspector.asset.rating ? "★" : "☆"
                            color: inspector.hasAsset && index < inspector.asset.rating
                                ? "#d89a16" : Theme.textDisabled
                            font.pixelSize: 15

                            MouseArea {
                                anchors.fill: parent
                                anchors.margins: -2
                                cursorShape: Qt.PointingHandCursor
                                onClicked: inspector.affinityRequested(
                                    inspector.asset,
                                    inspector.asset.liked,
                                    inspector.asset.rating === parent.index + 1
                                        ? 0 : parent.index + 1)
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    Layout.preferredHeight: 142
                    radius: 9
                    color: Theme.waveformSurface
                    border.color: Theme.borderStrong

                    WaveformView {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        anchors.topMargin: 18
                        anchors.bottomMargin: 30
                        levels: inspector.waveformLevels
                        progress: inspector.hasAsset && player.duration > 0
                            && inspector.loadedPath === inspector.asset.path
                            ? player.position / player.duration : 0
                    }

                    EchoIconButton {
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: 9
                        anchors.bottomMargin: 7
                        source: inspector.hasAsset && player.isPlaying
                            && inspector.loadedPath === inspector.asset.path
                            ? "qrc:/EchoDesktop/icons/pause.svg"
                            : "qrc:/EchoDesktop/icons/play.svg"
                        toolTipText: player.isPlaying ? qsTr("Pause") : qsTr("Play")
                        enabled: inspector.hasAsset
                            && inspector.asset.pathStatus !== "missing"
                        buttonSize: 27
                        iconSize: 14
                        onClicked: inspector.playSelected()
                    }

                    Text {
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: 10
                        anchors.bottomMargin: 12
                        text: inspector.hasAsset
                            && inspector.loadedPath === inspector.asset.path
                            ? inspector.formatDuration(player.position) + " / "
                                + inspector.formatDuration(player.duration)
                            : inspector.hasAsset
                                ? inspector.formatDuration(inspector.asset.durationMillis) : "—"
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("SOUND ATTRIBUTES")
                    evidence: qsTr("Model evidence")

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 9
                        rowSpacing: 7

                        Text { text: qsTr("Event"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.asset.eventType.length > 0
                                ? inspector.asset.eventType : qsTr("Not inferred")
                            color: inspector.asset.eventType.length > 0
                                ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text { text: qsTr("Mood"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.asset.mood.length > 0
                                ? inspector.asset.mood : qsTr("Not inferred")
                            color: inspector.asset.mood.length > 0
                                ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("AI KEYWORDS")
                    evidence: qsTr("Model evidence")

                    Flow {
                        Layout.fillWidth: true
                        spacing: 5

                        Repeater {
                            model: inspector.keywordTags()

                            delegate: Rectangle {
                                required property var modelData
                                width: keywordText.implicitWidth + 12
                                height: 23
                                radius: 6
                                color: Theme.accentSurfaceQuiet

                                Text {
                                    id: keywordText
                                    anchors.centerIn: parent
                                    text: String(modelData)
                                    color: Theme.accentSelectionText
                                    font.pixelSize: Theme.fontMeta
                                }
                            }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: inspector.asset.keywords.length === 0
                        text: qsTr("No keywords have been extracted yet.")
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                        wrapMode: Text.WordWrap
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("TEXT")
                    evidence: inspector.textRecords.length > 0
                        ? qsTr("Model evidence") : ""

                    Text {
                        Layout.fillWidth: true
                        text: inspector.textRecords.length > 0
                            ? inspector.textRecords[0].text
                            : inspector.jobStats.pending > 0 || inspector.jobStats.running > 0
                                ? qsTr("Echo is extracting text in the background…")
                                : qsTr("No text has been extracted from this sound yet.")
                        color: inspector.textRecords.length > 0
                            ? Theme.textPrimary : Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                        lineHeight: 1.35
                        wrapMode: Text.WordWrap
                        maximumLineCount: 6
                        elide: Text.ElideRight
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("SOURCE METADATA")
                    evidence: qsTr("Original evidence")

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 9
                        rowSpacing: 7

                        Text { text: qsTr("Recorded"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset
                                && inspector.asset.sourceCreatedAt.length > 0
                                ? inspector.asset.sourceCreatedAt
                                : inspector.hasAsset
                                    ? inspector.formatDate(inspector.asset.recordedAtMillis > 0
                                    ? inspector.asset.recordedAtMillis
                                    : inspector.asset.importedAtMillis) : ""
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text { text: qsTr("Location"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset
                                && inspector.asset.sourceLocation.length > 0
                                ? inspector.asset.sourceLocation : qsTr("Not embedded")
                            color: inspector.hasAsset
                                && inspector.asset.sourceLocation.length > 0
                                ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text { text: qsTr("Format"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset
                                ? inspector.asset.codec.toUpperCase()
                                    + (inspector.asset.containerFormat.length > 0
                                        ? " · " + inspector.asset.containerFormat : "")
                                : ""
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text { text: qsTr("Audio"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset && inspector.asset.sampleRate > 0
                                ? qsTr("%1 Hz · %2 channel(s)")
                                    .arg(inspector.asset.sampleRate)
                                    .arg(inspector.asset.channelCount)
                                : qsTr("Technical metadata pending")
                            color: inspector.hasAsset && inspector.asset.sampleRate > 0
                                ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("ORIGINAL")

                    Text {
                        Layout.fillWidth: true
                        text: inspector.hasAsset ? inspector.asset.path : ""
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WrapAnywhere
                        maximumLineCount: 3
                        elide: Text.ElideMiddle
                    }
                }

                Item { Layout.preferredHeight: 10 }
            }
        }
    }
}
