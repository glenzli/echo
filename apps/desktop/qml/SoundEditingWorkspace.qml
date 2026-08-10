//! Dedicated sound-adjustment workspace. It coordinates immutable-source
//! playback, draft audition, selection looping, and explicit publication;
//! timeline gestures and draft history remain in their semantic owners.

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
    readonly property bool dirty: adjustmentDraft.dirty
    readonly property bool canUndo: adjustmentDraft.canUndo
    readonly property bool canRedo: adjustmentDraft.canRedo

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
        return prefix + ":" + adjustmentDraft.trimStartMillis + ":"
            + adjustmentDraft.trimEndMillis + ":"
            + (auditionOriginal ? 0 : adjustmentDraft.fadeInMillis) + ":"
            + (auditionOriginal ? 0 : adjustmentDraft.fadeOutMillis) + ":"
            + (auditionOriginal ? 0 : adjustmentDraft.fadeInCurve) + ":"
            + (auditionOriginal ? 0 : adjustmentDraft.fadeOutCurve) + ":"
            + (auditionOriginal ? 0 : adjustmentDraft.gainCentibels) + ":"
            + (auditionOriginal ? 0 : adjustmentDraft.lowCutHertz)
    }

    function refreshAsset() : void {
        waveformLevels = []
        if (!asset || !asset.id || asset.pathStatus === "missing") return
        waveformLevels = backend.waveformForAsset(asset.id)
    }

    function playFrom(millis: int) : void {
        if (!asset || asset.pathStatus === "missing") return
        player.playAdjusted(asset.path,
                            adjustmentDraft.trimStartMillis,
                            adjustmentDraft.trimEndMillis,
                            auditionOriginal ? 0 : adjustmentDraft.fadeInMillis,
                            auditionOriginal ? 0 : adjustmentDraft.fadeOutMillis,
                            auditionOriginal ? 0 : adjustmentDraft.fadeInCurve,
                            auditionOriginal ? 0 : adjustmentDraft.fadeOutCurve,
                            auditionOriginal ? 0 : adjustmentDraft.gainCentibels,
                            auditionOriginal ? 0 : adjustmentDraft.lowCutHertz)
        loadedPath = asset.path
        loadedAdjustmentKey = adjustmentKey()
        const start = Math.max(adjustmentDraft.trimStartMillis,
            Math.min(millis, adjustmentDraft.trimEndMillis))
        if (start > adjustmentDraft.trimStartMillis) player.seek(start)
    }

    function ownsActivePlayback() : bool {
        return player.active && hasAsset && loadedPath === asset.path
            && loadedAdjustmentKey === adjustmentKey()
    }

    function seekOrLoad(millis: int) : void {
        if (!hasAsset) return
        if (!ownsActivePlayback()) {
            playFrom(millis)
        } else {
            player.seek(Math.max(adjustmentDraft.trimStartMillis,
                Math.min(millis, adjustmentDraft.trimEndMillis)))
        }
    }

    function defaultPlaybackStart() : int {
        return editorTimeline.hasTimeSelection
            ? editorTimeline.selectionStartMillis
            : adjustmentDraft.trimStartMillis
    }

    function togglePlayback() : void {
        if (!hasAsset || asset.pathStatus === "missing") return
        if (!ownsActivePlayback()) {
            playFrom(defaultPlaybackStart())
        } else {
            player.togglePause()
        }
    }

    function setOriginalAudition(enabled: bool) : void {
        if (auditionOriginal === enabled) return
        const resumeAt = loadedPath === (hasAsset ? asset.path : "")
            ? player.position : defaultPlaybackStart()
        const wasLoaded = ownsActivePlayback()
        auditionOriginal = enabled
        if (wasLoaded) playFrom(resumeAt)
    }

    function undo() : void {
        adjustmentDraft.undo()
    }

    function redo() : void {
        adjustmentDraft.redo()
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

    SoundAdjustmentDraft {
        id: adjustmentDraft

        asset: workspace.asset

        onSaveRequested: function(startMillis, endMillis, fadeIn, fadeOut,
                                  fadeInCurve, fadeOutCurve, gain, lowCut) {
            if (backend.setAssetAdjustment(workspace.asset.id, startMillis, endMillis,
                                           fadeIn, fadeOut, fadeInCurve,
                                           fadeOutCurve, gain, lowCut)) {
                adjustmentDraft.markSaved()
                workspace.auditionOriginal = false
                workspace.loadedAdjustmentKey = ""
            }
        }
    }

    Timer {
        interval: 40
        repeat: true
        running: workspace.hasAsset && player.playing
            && editorTimeline.loopSelection && editorTimeline.hasTimeSelection
            && workspace.loadedPath === workspace.asset.path
        onTriggered: {
            if (player.position >= editorTimeline.selectionEndMillis - 40) {
                player.seek(editorTimeline.selectionStartMillis)
            }
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
        anchors.margins: 16
        anchors.topMargin: 12
        spacing: 10
        visible: workspace.hasAsset

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 36
            spacing: 10

            Text {
                Layout.maximumWidth: Math.min(440, implicitWidth)
                text: workspace.hasAsset ? workspace.fileName(workspace.asset.path) : ""
                color: Theme.textPrimary
                font.pixelSize: 16
                font.weight: Font.DemiBold
                elide: Text.ElideRight

                HoverHandler { id: sourceHover }
                ToolTip.visible: sourceHover.hovered
                ToolTip.text: workspace.hasAsset ? workspace.asset.path : ""
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 16
                color: Theme.border
            }

            Text {
                text: workspace.technicalDetails()
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }

            Item { Layout.fillWidth: true }
        }

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Vertical

            handle: Rectangle {
                implicitHeight: 7
                color: Theme.window

                Rectangle {
                    anchors.centerIn: parent
                    width: 44
                    height: 1
                    color: Theme.borderStrong
                }
            }

            SoundEditorTimeline {
                id: editorTimeline

                SplitView.fillWidth: true
                SplitView.fillHeight: true
                SplitView.minimumHeight: 260
                waveformLevels: workspace.waveformLevels
                sourceDurationMillis: adjustmentDraft.sourceDurationMillis
                trimStartMillis: adjustmentDraft.trimStartMillis
                trimEndMillis: adjustmentDraft.trimEndMillis
                fadeInMillis: adjustmentDraft.fadeInMillis
                fadeOutMillis: adjustmentDraft.fadeOutMillis
                fadeInCurve: adjustmentDraft.fadeInCurve
                fadeOutCurve: adjustmentDraft.fadeOutCurve
                gainCentibels: adjustmentDraft.gainCentibels
                playbackPositionMillis: workspace.hasAsset
                        && workspace.ownsActivePlayback()
                    ? player.position : adjustmentDraft.trimStartMillis
                isPlaying: workspace.ownsActivePlayback() && player.playing
                enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"

                onTrimRequested: function(startMillis, endMillis) {
                    adjustmentDraft.setTrimRange(startMillis, endMillis)
                }
                onFadeRequested: function(fadeIn, fadeOut) {
                    adjustmentDraft.setFades(fadeIn, fadeOut)
                }
                onGainRequested: gain => adjustmentDraft.setGain(gain)
                onSeekRequested: millis => workspace.seekOrLoad(millis)
                onPlayPauseRequested: workspace.togglePlayback()
                onEditGestureStarted: adjustmentDraft.beginGesture()
                onEditGestureFinished: adjustmentDraft.endGesture()
                onUndoRequested: adjustmentDraft.undo()
                onRedoRequested: adjustmentDraft.redo()
            }

            SoundAdjustmentEditor {
                SplitView.fillWidth: true
                SplitView.preferredHeight: 189
                SplitView.minimumHeight: 181
                SplitView.maximumHeight: 230
                draft: adjustmentDraft
                hasTimeSelection: editorTimeline.hasTimeSelection
                selectionStartMillis: editorTimeline.selectionStartMillis
                selectionEndMillis: editorTimeline.selectionEndMillis
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            spacing: 10

            EchoIconButton {
                source: workspace.hasAsset && player.playing
                        && workspace.ownsActivePlayback()
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
                enabled: workspace.ownsActivePlayback()
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
    }
}
