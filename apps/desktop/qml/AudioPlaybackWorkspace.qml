//! Primary selected-recording workspace. Waveform, transport, and transcript
//! evidence stay together because they share playback and asset lifecycle.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: preview

    property var asset: null
    property var waveformLevels: []
    property string loadedPath: ""

    color: Theme.panelRaised
    radius: Theme.panelRadius
    border.color: Theme.border

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function formatDuration(millis: int) : string {
        if (!millis || millis <= 0) {
            return "0:00"
        }
        const totalSeconds = Math.floor(millis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds
    }

    function formatTimestamp(seconds: real) : string {
        const minutes = Math.floor(seconds / 60)
        const rest = seconds - minutes * 60
        return minutes + ":" + (rest < 10 ? "0" : "") + rest.toFixed(1)
    }

    function refreshAsset() : void {
        transcriptModel.clear()
        waveformLevels = []
        if (asset === null || !asset.id || asset.pathStatus === "missing") {
            return
        }
        waveformLevels = backend.waveformForAsset(asset.id)
        const records = backend.transcriptsForAsset(asset.id)
        if (records.length > 0) {
            for (const segment of records[0].segments) {
                transcriptModel.append(segment)
            }
        }
    }

    function playFrom(millis: int) : void {
        if (asset === null || asset.pathStatus === "missing") {
            return
        }
        player.play(asset.path)
        loadedPath = asset.path
        if (millis > 0) {
            player.seek(millis)
        }
    }

    onAssetChanged: Qt.callLater(refreshAsset)

    Connections {
        target: backend

        function onTranscriptionFinished(assetId: string, ok: bool, message: string) : void {
            if (ok && preview.asset !== null && assetId === preview.asset.id) {
                preview.refreshAsset()
            }
            if (!ok) {
                console.warn("transcription failed: " + message)
            }
        }
    }

    ListModel {
        id: transcriptModel
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 380)
        spacing: 12
        visible: preview.asset === null

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 68
            Layout.preferredHeight: 68
            radius: 22
            color: Theme.surfaceSubtle

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                size: 30
                color: Theme.textDisabled
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a recording")
            color: Theme.textPrimary
            font.pixelSize: 18
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("The waveform, playback controls, and transcript will stay at the center of your workspace.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 12
        visible: preview.asset !== null

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3

                Text {
                    Layout.fillWidth: true
                    text: preview.asset !== null ? preview.fileName(preview.asset.path) : ""
                    color: Theme.textPrimary
                    font.pixelSize: 20
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: preview.asset !== null ? preview.asset.path : ""
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideMiddle
                }
            }

            Rectangle {
                Layout.preferredWidth: statusText.implicitWidth + 16
                Layout.preferredHeight: 24
                radius: 12
                color: preview.asset !== null && preview.asset.pathStatus === "missing"
                    ? Theme.warningSurface : Theme.accentSurfaceQuiet

                Text {
                    id: statusText
                    anchors.centerIn: parent
                    text: preview.asset !== null && preview.asset.pathStatus === "missing"
                        ? qsTr("Missing") : qsTr("Original preserved")
                    color: preview.asset !== null && preview.asset.pathStatus === "missing"
                        ? Theme.warningText : Theme.accentSelectionText
                    font.pixelSize: Theme.fontMeta
                }
            }
        }

        AudioMetadataBar {
            Layout.fillWidth: true
            Layout.preferredHeight: implicitHeight
            asset: preview.asset
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.minimumHeight: 220
            Layout.preferredHeight: Math.max(260, Math.min(380, preview.height * 0.43))
            radius: 12
            color: Theme.waveformSurface
            border.color: Theme.borderStrong

            WaveformView {
                anchors.fill: parent
                anchors.margins: 18
                levels: preview.waveformLevels
                progress: player.duration > 0 ? player.position / player.duration : 0
            }

            MouseArea {
                anchors.fill: parent
                enabled: player.duration > 0 && preview.loadedPath === preview.asset.path
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: function(mouse) {
                    player.seek(Math.round(mouse.x / width * player.duration))
                }
            }

            Text {
                anchors.centerIn: parent
                visible: preview.waveformLevels.length === 0
                text: preview.asset !== null && preview.asset.pathStatus === "missing"
                    ? qsTr("Original file is unavailable") : qsTr("Preparing waveform…")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            EchoIconButton {
                source: player.isPlaying && preview.loadedPath === preview.asset.path
                    ? "qrc:/EchoDesktop/icons/pause.svg"
                    : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: player.isPlaying ? qsTr("Pause") : qsTr("Play")
                enabled: preview.asset !== null && preview.asset.pathStatus !== "missing"
                buttonSize: 38
                iconSize: 19
                onClicked: {
                    if (preview.loadedPath !== preview.asset.path) {
                        preview.playFrom(0)
                    } else {
                        player.togglePause()
                    }
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/stop.svg"
                toolTipText: qsTr("Stop")
                enabled: preview.loadedPath === preview.asset.path
                buttonSize: 34
                iconSize: 16
                onClicked: player.stop()
            }

            Text {
                text: preview.formatDuration(player.position) + " / "
                    + preview.formatDuration(player.duration)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Slider {
                Layout.fillWidth: true
                from: 0
                to: Math.max(1, player.duration)
                value: player.duration > 0 ? player.position : 0
                enabled: player.duration > 0 && preview.loadedPath === preview.asset.path
                onMoved: player.seek(value)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Transcript")
                hint: transcriptModel.count > 0
                    ? qsTr("Click a segment to seek")
                    : qsTr("No transcript evidence yet")
            }

            EchoButton {
                visible: transcriptModel.count === 0
                text: backend.transcribing ? qsTr("Analyzing…") : qsTr("Analyze (prototype)")
                enabled: !backend.transcribing && preview.asset !== null
                    && preview.asset.pathStatus !== "missing"
                ghost: true
                onClicked: backend.transcribeAsset(
                    preview.asset.id, modelPrefs.modelRoot,
                    modelPrefs.python, modelPrefs.workerScript)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 120
            radius: Theme.controlRadius
            color: Theme.surfaceSubtle
            border.color: Theme.border

            Text {
                anchors.centerIn: parent
                width: parent.width - 48
                visible: transcriptModel.count === 0
                text: qsTr("Transcription is optional. The direct local worker remains a temporary compatibility path until Infer Build integration.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            ListView {
                id: transcriptList

                anchors.fill: parent
                anchors.margins: 7
                visible: transcriptModel.count > 0
                spacing: 3
                clip: true
                model: transcriptModel

                delegate: Rectangle {
                    required property var modelData

                    width: transcriptList.width
                    height: Math.max(38, segmentText.implicitHeight + 14)
                    radius: Theme.compactControlRadius
                    color: player.duration > 0
                        && player.position >= modelData.start * 1000
                        && player.position <= modelData.end * 1000
                        ? Theme.accentSurface : Theme.transparent

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (preview.loadedPath !== preview.asset.path) {
                                preview.playFrom(modelData.start * 1000)
                            } else {
                                player.seek(modelData.start * 1000)
                            }
                        }
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 12

                        Text {
                            text: preview.formatTimestamp(modelData.start)
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            Layout.alignment: Qt.AlignTop
                            Layout.topMargin: 2
                        }

                        Text {
                            id: segmentText
                            Layout.fillWidth: true
                            text: modelData.text
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            wrapMode: Text.WordWrap
                        }
                    }
                }

                ScrollBar.vertical: ScrollBar {}
            }
        }
    }
}
