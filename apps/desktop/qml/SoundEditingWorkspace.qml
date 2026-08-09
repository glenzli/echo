//! Dedicated non-destructive editing workspace. It owns adjustment drafts,
//! range visualization, adjusted preview, and explicit version publication;
//! Audio Space remains a listening and evidence-browsing surface.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: workspace

    required property var asset

    property var waveformLevels: []
    property string loadedPath: ""
    property string loadedAdjustmentKey: ""

    readonly property bool hasAsset: asset !== null && asset !== undefined

    color: Theme.window

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function formatDuration(millis: int) : string {
        const safeMillis = Math.max(0, millis)
        const totalSeconds = Math.floor(safeMillis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        const tenths = Math.floor((safeMillis % 1000) / 100)
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds + "." + tenths
    }

    function adjustmentKey() : string {
        return adjustmentEditor.trimStartMillis + ":" + adjustmentEditor.trimEndMillis
            + ":" + adjustmentEditor.fadeInMillis + ":" + adjustmentEditor.fadeOutMillis
            + ":" + adjustmentEditor.gainCentibels
    }

    function refreshAsset() : void {
        waveformLevels = []
        if (!asset || !asset.id || asset.pathStatus === "missing") {
            return
        }
        waveformLevels = backend.waveformForAsset(asset.id)
    }

    function playFrom(millis: int) : void {
        if (!asset || asset.pathStatus === "missing") {
            return
        }
        player.playAdjusted(asset.path,
                            adjustmentEditor.trimStartMillis,
                            adjustmentEditor.trimEndMillis,
                            adjustmentEditor.fadeInMillis,
                            adjustmentEditor.fadeOutMillis,
                            adjustmentEditor.gainCentibels)
        loadedPath = asset.path
        loadedAdjustmentKey = adjustmentKey()
        const start = Math.max(adjustmentEditor.trimStartMillis,
                               Math.min(millis, adjustmentEditor.trimEndMillis))
        if (start > adjustmentEditor.trimStartMillis) {
            player.seek(start)
        }
    }

    onAssetChanged: {
        player.stop()
        loadedPath = ""
        loadedAdjustmentKey = ""
        Qt.callLater(refreshAsset)
    }

    Connections {
        target: backend

        function onAssetsChanged() : void {
            workspace.refreshAsset()
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 420)
        spacing: 12
        visible: !workspace.hasAsset

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 72
            Layout.preferredHeight: 72
            radius: 24
            color: Theme.surfaceSubtle

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/edit.svg"
                size: 30
                color: Theme.textDisabled
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a sound in Audio Space")
            color: Theme.textPrimary
            font.pixelSize: 20
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Choose a sound before opening adjustments.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 16
        visible: workspace.hasAsset

        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                Text {
                    text: qsTr("Sound Adjustments")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                }

                Text {
                    Layout.fillWidth: true
                    text: workspace.hasAsset ? workspace.fileName(workspace.asset.path) : ""
                    color: Theme.textPrimary
                    font.pixelSize: 24
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: workspace.hasAsset ? workspace.asset.path : ""
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideMiddle
                }
            }

        }

        AudioMetadataBar {
            Layout.fillWidth: true
            Layout.preferredHeight: implicitHeight
            asset: workspace.asset
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 250
            radius: 16
            color: Theme.waveformSurface
            border.color: Theme.borderStrong

            WaveformView {
                anchors.fill: parent
                anchors.margins: 28
                levels: workspace.waveformLevels
                progress: workspace.hasAsset && workspace.loadedPath === workspace.asset.path
                        && player.duration > 0
                    ? player.position / player.duration : 0
            }

            Rectangle {
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                width: parent.width * adjustmentEditor.trimStartMillis
                    / Math.max(1, adjustmentEditor.sourceDurationMillis)
                color: Theme.window
                opacity: 0.62
            }

            Rectangle {
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.right: parent.right
                width: parent.width * Math.max(0,
                    adjustmentEditor.sourceDurationMillis - adjustmentEditor.trimEndMillis)
                    / Math.max(1, adjustmentEditor.sourceDurationMillis)
                color: Theme.window
                opacity: 0.62
            }

            Rectangle {
                x: parent.width * adjustmentEditor.trimStartMillis
                    / Math.max(1, adjustmentEditor.sourceDurationMillis)
                width: parent.width * adjustmentEditor.selectedDurationMillis
                    / Math.max(1, adjustmentEditor.sourceDurationMillis)
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                color: Theme.transparent
                border.width: adjustmentEditor.dirty ? 2 : 1
                border.color: Theme.accent
            }

            MouseArea {
                anchors.fill: parent
                enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"
                    && adjustmentEditor.sourceDurationMillis > 0
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: function(mouse) {
                    const target = Math.round(mouse.x / width
                        * adjustmentEditor.sourceDurationMillis)
                    if (workspace.loadedPath !== workspace.asset.path
                            || workspace.loadedAdjustmentKey !== workspace.adjustmentKey()) {
                        workspace.playFrom(target)
                    } else {
                        player.seek(Math.max(adjustmentEditor.trimStartMillis,
                            Math.min(target, adjustmentEditor.trimEndMillis)))
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: workspace.waveformLevels.length === 0
                text: workspace.hasAsset && workspace.asset.pathStatus === "missing"
                    ? qsTr("Original file is unavailable") : qsTr("Preparing waveform…")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            EchoIconButton {
                source: workspace.hasAsset && player.isPlaying
                        && workspace.loadedPath === workspace.asset.path
                    ? "qrc:/EchoDesktop/icons/pause.svg"
                    : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: player.isPlaying ? qsTr("Pause") : qsTr("Preview")
                enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"
                buttonSize: 40
                iconSize: 19
                onClicked: {
                    if (workspace.loadedPath !== workspace.asset.path
                            || workspace.loadedAdjustmentKey !== workspace.adjustmentKey()) {
                        workspace.playFrom(adjustmentEditor.trimStartMillis)
                    } else {
                        player.togglePause()
                    }
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/stop.svg"
                toolTipText: qsTr("Stop")
                enabled: workspace.hasAsset
                    && workspace.loadedPath === workspace.asset.path
                buttonSize: 36
                iconSize: 16
                onClicked: player.stop()
            }

            Text {
                text: workspace.formatDuration(player.position) + " / "
                    + workspace.formatDuration(player.duration)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 104
            }

            Slider {
                Layout.fillWidth: true
                from: adjustmentEditor.trimStartMillis
                to: Math.max(adjustmentEditor.trimStartMillis + 1,
                             adjustmentEditor.trimEndMillis)
                value: player.duration > 0 ? player.position : 0
                enabled: workspace.hasAsset && player.duration > 0
                    && workspace.loadedPath === workspace.asset.path
                onMoved: player.seek(value)
            }

            Text {
                text: qsTr("Selected %1").arg(
                    workspace.formatDuration(adjustmentEditor.selectedDurationMillis))
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }
        }

        SoundAdjustmentEditor {
            id: adjustmentEditor

            Layout.fillWidth: true
            asset: workspace.asset

            onPreviewRequested: function(startMillis, endMillis, fadeIn, fadeOut, gain) {
                workspace.playFrom(startMillis)
            }
            onSaveRequested: function(startMillis, endMillis, fadeIn, fadeOut, gain) {
                if (backend.setAssetAdjustment(workspace.asset.id, startMillis, endMillis,
                                               fadeIn, fadeOut, gain)) {
                    workspace.playFrom(startMillis)
                }
            }
        }
    }
}
