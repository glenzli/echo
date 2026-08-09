//! Dedicated editing workspace. It owns selected-source lifecycle, adjusted
//! audition, original comparison, transport, and version publication while
//! SoundEditorTimeline owns direct manipulation of the adjustment draft.

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
    property bool auditionOriginal: false

    readonly property bool hasAsset: asset !== null && asset !== undefined

    color: Theme.window

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function formatDuration(millis: int) : string {
        const safeMillis = Math.max(0, millis)
        const totalSeconds = Math.floor(safeMillis / 1000)
        const hours = Math.floor(totalSeconds / 3600)
        const minutes = Math.floor((totalSeconds % 3600) / 60)
        const seconds = totalSeconds % 60
        const tenths = Math.floor((safeMillis % 1000) / 100)
        if (hours > 0) {
            return hours + ":" + String(minutes).padStart(2, "0") + ":"
                + String(seconds).padStart(2, "0") + "." + tenths
        }
        return minutes + ":" + String(seconds).padStart(2, "0") + "." + tenths
    }

    function technicalDetails() : string {
        if (!hasAsset) return ""
        const values = []
        if (asset.codec.length > 0) values.push(asset.codec.toUpperCase())
        if (asset.sampleRate > 0) {
            values.push((asset.sampleRate / 1000).toFixed(
                asset.sampleRate % 1000 === 0 ? 0 : 1) + " kHz")
        }
        if (asset.channelCount > 0) {
            values.push(qsTr("%1 channels").arg(asset.channelCount))
        }
        values.push(formatDuration(asset.durationMillis))
        return values.join(" · ")
    }

    function adjustmentKey() : string {
        const prefix = auditionOriginal ? "original" : "adjusted"
        return prefix + ":" + adjustmentEditor.trimStartMillis + ":"
            + adjustmentEditor.trimEndMillis + ":"
            + (auditionOriginal ? 0 : adjustmentEditor.fadeInMillis) + ":"
            + (auditionOriginal ? 0 : adjustmentEditor.fadeOutMillis) + ":"
            + (auditionOriginal ? 0 : adjustmentEditor.gainCentibels)
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
                            auditionOriginal ? 0 : adjustmentEditor.fadeInMillis,
                            auditionOriginal ? 0 : adjustmentEditor.fadeOutMillis,
                            auditionOriginal ? 0 : adjustmentEditor.gainCentibels)
        loadedPath = asset.path
        loadedAdjustmentKey = adjustmentKey()
        const start = Math.max(adjustmentEditor.trimStartMillis,
                               Math.min(millis, adjustmentEditor.trimEndMillis))
        if (start > adjustmentEditor.trimStartMillis) {
            player.seek(start)
        }
    }

    function seekOrLoad(millis: int) : void {
        if (!hasAsset) return
        if (loadedPath !== asset.path || loadedAdjustmentKey !== adjustmentKey()) {
            playFrom(millis)
        } else {
            player.seek(Math.max(adjustmentEditor.trimStartMillis,
                Math.min(millis, adjustmentEditor.trimEndMillis)))
        }
    }

    function togglePlayback() : void {
        if (!hasAsset || asset.pathStatus === "missing") return
        if (loadedPath !== asset.path || loadedAdjustmentKey !== adjustmentKey()) {
            playFrom(adjustmentEditor.trimStartMillis)
        } else {
            player.togglePause()
        }
    }

    function setOriginalAudition(enabled: bool) : void {
        if (auditionOriginal === enabled) return
        const resumeAt = loadedPath === (hasAsset ? asset.path : "")
            ? player.position : adjustmentEditor.trimStartMillis
        const wasLoaded = hasAsset && loadedPath === asset.path
        auditionOriginal = enabled
        if (wasLoaded) {
            playFrom(resumeAt)
        }
    }

    onAssetChanged: {
        player.stop()
        auditionOriginal = false
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
        anchors.margins: 20
        spacing: 12
        visible: workspace.hasAsset

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 54
            spacing: 14

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3

                Text {
                    Layout.fillWidth: true
                    text: workspace.hasAsset ? workspace.fileName(workspace.asset.path) : ""
                    color: Theme.textPrimary
                    font.pixelSize: 22
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: workspace.technicalDetails()
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideRight
                }
            }

            Text {
                Layout.maximumWidth: 380
                text: workspace.hasAsset ? workspace.asset.path : ""
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideMiddle
            }
        }

        SoundEditorTimeline {
            id: editorTimeline

            Layout.fillWidth: true
            Layout.fillHeight: true
            waveformLevels: workspace.waveformLevels
            sourceDurationMillis: adjustmentEditor.sourceDurationMillis
            trimStartMillis: adjustmentEditor.trimStartMillis
            trimEndMillis: adjustmentEditor.trimEndMillis
            fadeInMillis: adjustmentEditor.fadeInMillis
            fadeOutMillis: adjustmentEditor.fadeOutMillis
            gainCentibels: adjustmentEditor.gainCentibels
            playbackPositionMillis: workspace.hasAsset
                    && workspace.loadedPath === workspace.asset.path
                ? player.position : adjustmentEditor.trimStartMillis
            isPlaying: workspace.hasAsset && workspace.loadedPath === workspace.asset.path
                && player.playing
            enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"

            onTrimRequested: function(startMillis, endMillis) {
                adjustmentEditor.setTrimRange(startMillis, endMillis)
            }
            onFadeRequested: function(fadeIn, fadeOut) {
                adjustmentEditor.setFades(fadeIn, fadeOut)
            }
            onGainRequested: gain => adjustmentEditor.setGain(gain)
            onSeekRequested: millis => workspace.seekOrLoad(millis)
            onPlayPauseRequested: workspace.togglePlayback()
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            spacing: 10

            EchoIconButton {
                source: workspace.hasAsset && player.playing
                        && workspace.loadedPath === workspace.asset.path
                    ? "qrc:/EchoDesktop/icons/pause.svg"
                    : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: player.playing ? qsTr("Pause") : qsTr("Play")
                enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"
                buttonSize: 40
                iconSize: 19
                onClicked: workspace.togglePlayback()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/stop.svg"
                toolTipText: qsTr("Stop")
                enabled: workspace.hasAsset && workspace.loadedPath === workspace.asset.path
                buttonSize: 36
                iconSize: 16
                onClicked: player.stop()
            }

            Text {
                text: workspace.formatDuration(player.position) + " / "
                    + workspace.formatDuration(player.duration)
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.bold: true
                Layout.preferredWidth: 124
            }

            Item { Layout.fillWidth: true }

            Text {
                text: qsTr("Audition")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            EchoButton {
                text: qsTr("Adjusted")
                ghost: true
                selected: !workspace.auditionOriginal
                onClicked: workspace.setOriginalAudition(false)
            }

            EchoButton {
                text: qsTr("Original")
                ghost: true
                selected: workspace.auditionOriginal
                onClicked: workspace.setOriginalAudition(true)
            }
        }

        SoundAdjustmentEditor {
            id: adjustmentEditor

            Layout.fillWidth: true
            asset: workspace.asset

            onSaveRequested: function(startMillis, endMillis, fadeIn, fadeOut, gain) {
                if (backend.setAssetAdjustment(workspace.asset.id, startMillis, endMillis,
                                               fadeIn, fadeOut, gain)) {
                    workspace.auditionOriginal = false
                    workspace.loadedAdjustmentKey = ""
                }
            }
        }
    }
}
