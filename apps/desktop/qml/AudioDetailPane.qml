//! Selected recording detail: waveform, playback, metadata, and transcript
//! evidence. The pane remains present as an empty-state anchor so the Audio
//! Space layout does not jump when selection changes.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: detail

    property var asset: null
    property var waveformLevels: []
    property string loadedPath: ""

    color: Theme.panel
    radius: Theme.panelRadius
    border.color: Theme.border

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function formatDuration(millis: int) : string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown length")
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
            if (ok && detail.asset !== null && assetId === detail.asset.id) {
                detail.refreshAsset()
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
        width: Math.min(parent.width - 48, 240)
        spacing: 10
        visible: detail.asset === null

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 52
            Layout.preferredHeight: 52
            radius: 16
            color: Theme.surfaceSubtle

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                size: 24
                color: Theme.textDisabled
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a recording")
            color: Theme.textPrimary
            font.pixelSize: 15
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Its waveform, playback controls, and transcript will appear here.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12
        visible: detail.asset !== null

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    Layout.fillWidth: true
                    text: detail.asset !== null ? detail.fileName(detail.asset.path) : ""
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.bold: true
                    elide: Text.ElideRight
                }

                Rectangle {
                    visible: detail.asset !== null && detail.asset.pathStatus === "missing"
                    Layout.preferredWidth: missingLabel.implicitWidth + 14
                    Layout.preferredHeight: 20
                    radius: 10
                    color: Theme.warningSurface

                    Text {
                        id: missingLabel
                        anchors.centerIn: parent
                        text: qsTr("Missing")
                        color: Theme.warningText
                        font.pixelSize: Theme.fontMeta
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                text: detail.asset !== null ? detail.asset.path : ""
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideMiddle
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            Repeater {
                model: detail.asset !== null ? [
                    detail.asset.codec ? detail.asset.codec.toUpperCase() : qsTr("Audio"),
                    detail.formatDuration(detail.asset.durationMillis),
                    detail.asset.maxLevel > 0
                        ? qsTr("Evidence L%1").arg(detail.asset.maxLevel)
                        : qsTr("Original")
                ] : []

                delegate: Rectangle {
                    required property var modelData

                    Layout.preferredWidth: chipText.implicitWidth + 14
                    Layout.preferredHeight: 22
                    radius: 11
                    color: Theme.surfaceSubtle

                    Text {
                        id: chipText
                        anchors.centerIn: parent
                        text: modelData
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }
            }

            Item { Layout.fillWidth: true }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 150
            radius: Theme.controlRadius
            color: Theme.waveformSurface
            border.color: Theme.border

            WaveformView {
                anchors.fill: parent
                anchors.margins: 10
                levels: detail.waveformLevels
                progress: player.duration > 0 ? player.position / player.duration : 0
            }

            Text {
                anchors.centerIn: parent
                visible: detail.waveformLevels.length === 0
                text: detail.asset !== null && detail.asset.pathStatus === "missing"
                    ? qsTr("Original file is unavailable") : qsTr("Preparing waveform…")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            EchoIconButton {
                source: player.isPlaying && detail.loadedPath === detail.asset.path
                    ? "qrc:/EchoDesktop/icons/pause.svg"
                    : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: player.isPlaying ? qsTr("Pause") : qsTr("Play")
                enabled: detail.asset !== null && detail.asset.pathStatus !== "missing"
                buttonSize: 36
                iconSize: 18
                onClicked: {
                    if (detail.loadedPath !== detail.asset.path) {
                        detail.playFrom(0)
                    } else {
                        player.togglePause()
                    }
                }
            }

            Text {
                text: detail.formatDuration(player.position) + " / "
                    + detail.formatDuration(player.duration)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Slider {
                Layout.fillWidth: true
                from: 0
                to: Math.max(1, player.duration)
                value: player.duration > 0 ? player.position : 0
                enabled: player.duration > 0 && detail.loadedPath === detail.asset.path
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
                enabled: !backend.transcribing && detail.asset !== null
                    && detail.asset.pathStatus !== "missing"
                ghost: true
                onClicked: backend.transcribeAsset(
                    detail.asset.id, modelPrefs.modelRoot,
                    modelPrefs.python, modelPrefs.workerScript)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 110
            radius: Theme.controlRadius
            color: Theme.surfaceSubtle
            border.color: Theme.border

            Text {
                anchors.centerIn: parent
                width: parent.width - 32
                visible: transcriptModel.count === 0
                text: qsTr("Transcription is optional. The direct local worker is a temporary compatibility path until Infer Build integration.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }

            ListView {
                id: transcriptList

                anchors.fill: parent
                anchors.margins: 6
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
                            if (detail.loadedPath !== detail.asset.path) {
                                detail.playFrom(modelData.start * 1000)
                            } else {
                                player.seek(modelData.start * 1000)
                            }
                        }
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        spacing: 10

                        Text {
                            text: detail.formatTimestamp(modelData.start)
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
