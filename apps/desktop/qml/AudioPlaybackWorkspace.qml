//! Listening owner for a selected catalog recording. This workspace presents
//! waveform, transport, and transcript evidence; authored adjustment drafts
//! live exclusively in SoundEditingWorkspace.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: preview

    property var asset: null
    property bool active: true
    property var waveformLevels: []
    property string loadedPath: ""
    property string loadedAdjustmentKey: ""
    property var jobStats: ({
            pending: 0,
            running: 0,
            done: 0,
            failed: 0
        })
    property var analysisStatus: ({
            stage: "text",
            state: "missing",
            recovery: "none",
            progress: 0,
            attempts: 0,
            errorCode: "",
            runtimeJobId: "",
            contractVersion: ""
        })

    readonly property bool hasAsset: asset !== null && asset !== undefined
    readonly property bool analysisActive: analysisStatus.state === "pending" || analysisStatus.state === "running"
    readonly property bool analysisFailed: analysisStatus.state === "failed" || analysisStatus.state === "cancelled"
    readonly property int sourceDurationMillis: hasAsset ? Number(asset.durationMillis) : 0
    readonly property int trimStartMillis: hasAsset ? Number(asset.trimStartMillis) : 0
    readonly property int trimEndMillis: hasAsset && Number(asset.trimEndMillis) > 0 ? Number(asset.trimEndMillis) : sourceDurationMillis
    readonly property int fadeInMillis: hasAsset ? Number(asset.fadeInMillis) : 0
    readonly property int fadeOutMillis: hasAsset ? Number(asset.fadeOutMillis) : 0
    readonly property int fadeInCurve: hasAsset ? Number(asset.fadeInCurve) : 0
    readonly property int fadeOutCurve: hasAsset ? Number(asset.fadeOutCurve) : 0
    readonly property int gainCentibels: hasAsset ? Number(asset.gainCentibels) : 0
    readonly property int lowCutHertz: hasAsset ? Number(asset.lowCutHertz) : 0
    readonly property var restorationValue: ({
            enabled: hasAsset ? Boolean(asset.restorationEnabled) : true,
            dePlosiveEnabled: hasAsset ? Boolean(asset.dePlosiveEnabled) : false,
            dePlosiveFrequencyHertz: hasAsset ? Number(asset.dePlosiveFrequencyHertz) : 140,
            dePlosiveSensitivityPercent: hasAsset ? Number(asset.dePlosiveSensitivityPercent) : 50,
            dePlosiveReductionCentibels: hasAsset ? Number(asset.dePlosiveReductionCentibels) : 1200,
            dePlosiveReleaseMillis: hasAsset ? Number(asset.dePlosiveReleaseMillis) : 160,
            noiseEnabled: hasAsset ? Boolean(asset.noiseReductionEnabled) : false,
            noiseReductionCentibels: hasAsset ? Number(asset.noiseReductionCentibels) : 900,
            noiseSensitivityPercent: hasAsset ? Number(asset.noiseReductionSensitivityPercent) : 50,
            noiseSmoothingMillis: hasAsset ? Number(asset.noiseReductionSmoothingMillis) : 240,
            deEsserEnabled: hasAsset ? Boolean(asset.deEsserEnabled) : false,
            deEsserFrequencyHertz: hasAsset ? Number(asset.deEsserFrequencyHertz) : 6500,
            deEsserThresholdCentibels: hasAsset ? Number(asset.deEsserThresholdCentibels) : -2400,
            deEsserReductionCentibels: hasAsset ? Number(asset.deEsserReductionCentibels) : 600
        })
    readonly property var deHumValue: ({
            enabled: hasAsset ? Boolean(asset.deHumEnabled) : false,
            fundamentalHertz: hasAsset ? Number(asset.deHumFundamentalHertz ?? 50) : 50,
            harmonicCount: hasAsset ? Number(asset.deHumHarmonicCount ?? 4) : 4,
            qualityTenths: hasAsset ? Number(asset.deHumQualityTenths ?? 300) : 300,
            depthCentibels: hasAsset ? Number(asset.deHumDepthCentibels ?? 2400) : 2400
        })
    readonly property var deClickValue: ({
            enabled: hasAsset ? Boolean(asset.deClickEnabled) : false,
            sensitivityPercent: hasAsset ? Number(asset.deClickSensitivityPercent ?? 50) : 50,
            maximumClickMicroseconds: hasAsset ? Number(asset.deClickMaximumClickMicroseconds ?? 1000) : 1000,
            repairPercent: hasAsset ? Number(asset.deClickRepairPercent ?? 100) : 100
        })
    readonly property var channelRepairValue: ({
            enabled: hasAsset ? Boolean(asset.channelRepairEnabled) : false,
            invertLeft: hasAsset ? Boolean(asset.channelRepairInvertLeft) : false,
            invertRight: hasAsset ? Boolean(asset.channelRepairInvertRight) : false,
            swapChannels: hasAsset ? Boolean(asset.channelRepairSwapChannels) : false,
            monoFoldDown: hasAsset ? Boolean(asset.channelRepairMonoFoldDown) : false,
            balancePercent: hasAsset ? Number(asset.channelRepairBalancePercent ?? 0) : 0
        })
    readonly property bool equalizerEnabled: hasAsset ? Boolean(asset.equalizerEnabled) : true
    readonly property var equalizerBands: hasAsset ? asset.equalizerBands : []
    readonly property bool compressorEnabled: hasAsset ? Boolean(asset.compressorEnabled) : false
    readonly property int compressorThresholdCentibels: hasAsset ? Number(asset.compressorThresholdCentibels) : -1800
    readonly property int compressorRatioTenths: hasAsset ? Number(asset.compressorRatioTenths) : 30
    readonly property int compressorAttackMillis: hasAsset ? Number(asset.compressorAttackMillis) : 10
    readonly property int compressorReleaseMillis: hasAsset ? Number(asset.compressorReleaseMillis) : 120
    readonly property int compressorMakeupCentibels: hasAsset ? Number(asset.compressorMakeupCentibels) : 0
    readonly property var reverbValue: ({
            character: hasAsset ? Number(asset.reverbCharacter) : 0,
            enabled: hasAsset ? Boolean(asset.reverbEnabled) : false,
            mixPercent: hasAsset ? Number(asset.reverbMixPercent) : 18,
            preDelayMillis: hasAsset ? Number(asset.reverbPreDelayMillis) : 20,
            decayMillis: hasAsset ? Number(asset.reverbDecayMillis) : 1800,
            sizePercent: hasAsset ? Number(asset.reverbSizePercent) : 55,
            dampingPercent: hasAsset ? Number(asset.reverbDampingPercent) : 45,
            lowCutHertz: hasAsset ? Number(asset.reverbLowCutHertz) : 120,
            highCutHertz: hasAsset ? Number(asset.reverbHighCutHertz) : 10000
        })
    readonly property bool limiterEnabled: hasAsset ? Boolean(asset.limiterEnabled) : false
    readonly property int limiterCeilingCentibels: hasAsset ? Number(asset.limiterCeilingCentibels) : -100
    readonly property int limiterReleaseMillis: hasAsset ? Number(asset.limiterReleaseMillis) : 100
    readonly property var effectChain: hasAsset ? asset.effectChain : [0, 1, 2, 3, 4]
    readonly property var editSegments: hasAsset ? asset.editSegments : []
    readonly property var effectMasks: hasAsset ? asset.effectMasks : []

    color: Theme.panelRaised
    border.color: Theme.border

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function formatDuration(millis: int): string {
        if (!millis || millis <= 0) {
            return "0:00";
        }
        const totalSeconds = Math.floor(millis / 1000);
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = totalSeconds % 60;
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds;
    }

    function formatTimestamp(seconds: real): string {
        const minutes = Math.floor(seconds / 60);
        const rest = seconds - minutes * 60;
        return minutes + ":" + (rest < 10 ? "0" : "") + rest.toFixed(1);
    }

    function analysisHint(): string {
        if (analysisStatus.recovery === "automatic")
            return qsTr("Waiting for Infer Runtime; analysis will resume automatically");
        if (analysisStatus.recovery === "source")
            return qsTr("Reconnect the Original to continue analysis");
        if (analysisStatus.recovery === "manual")
            return qsTr("Background analysis needs attention");
        if (analysisStatus.stage === "sound_events" && analysisActive)
            return qsTr("Identifying audible events in the background…");
        if (analysisStatus.stage === "contextual" && analysisActive)
            return qsTr("Understanding this sound in the background…");
        if (transcriptModel.count > 0)
            return analysisStatus.stage === "alignment" && analysisActive ? qsTr("Refining word timing in the background…") : qsTr("Click a segment to seek");
        if (analysisActive)
            return qsTr("Extracting text in the background…");
        return analysisStatus.stage === "complete" ? qsTr("No speech was detected in this sound") : qsTr("Text is extracted automatically");
    }

    function refreshAsset(): void {
        transcriptModel.clear();
        waveformLevels = [];
        if (!active) {
            return;
        }
        if (!asset || !asset.id || asset.pathStatus === "missing") {
            analysisStatus = ({
                    stage: "text",
                    state: "missing",
                    recovery: "none",
                    progress: 0,
                    attempts: 0,
                    errorCode: "",
                    runtimeJobId: "",
                    contractVersion: ""
                });
            return;
        }
        analysisStatus = backend.analysisStatusForAsset(asset.id);
        waveformLevels = backend.waveformForAsset(asset.id);
        const records = backend.transcriptsForAsset(asset.id);
        if (records.length > 0) {
            for (const segment of records[0].segments) {
                transcriptModel.append(segment);
            }
        }
    }

    function adjustmentKey(): string {
        return trimStartMillis + ":" + trimEndMillis + ":" + fadeInMillis + ":" + fadeOutMillis + ":" + fadeInCurve + ":" + fadeOutCurve + ":" + gainCentibels + ":" + lowCutHertz + ":" + JSON.stringify(restorationValue) + ":" + JSON.stringify(deHumValue) + ":" + JSON.stringify(deClickValue) + ":" + JSON.stringify(channelRepairValue) + ":" + equalizerEnabled + ":" + JSON.stringify(equalizerBands) + ":" + compressorEnabled + ":" + compressorThresholdCentibels + ":" + compressorRatioTenths + ":" + compressorAttackMillis + ":" + compressorReleaseMillis + ":" + compressorMakeupCentibels + ":" + JSON.stringify(reverbValue) + ":" + limiterEnabled + ":" + limiterCeilingCentibels + ":" + limiterReleaseMillis + ":" + JSON.stringify(effectChain) + ":" + JSON.stringify(editSegments) + ":" + JSON.stringify(effectMasks);
    }

    function playFrom(millis: int): void {
        if (!asset || asset.pathStatus === "missing") {
            return;
        }
        player.playAdjusted(asset.path, trimStartMillis, trimEndMillis, fadeInMillis, fadeOutMillis, fadeInCurve, fadeOutCurve, gainCentibels, lowCutHertz, restorationValue, deHumValue, deClickValue, channelRepairValue, equalizerEnabled, equalizerBands, compressorEnabled, compressorThresholdCentibels, compressorRatioTenths, compressorAttackMillis, compressorReleaseMillis, compressorMakeupCentibels, reverbValue, limiterEnabled, limiterCeilingCentibels, limiterReleaseMillis, effectChain, editSegments, effectMasks);
        loadedPath = asset.path;
        loadedAdjustmentKey = adjustmentKey();
        const start = Math.max(trimStartMillis, Math.min(millis, trimEndMillis));
        if (start > trimStartMillis) {
            player.seek(start);
        }
    }

    function ownsActivePlayback(): bool {
        return player.active && hasAsset && loadedPath === asset.path && loadedAdjustmentKey === adjustmentKey();
    }

    onAssetChanged: {
        if (asset && loadedPath.length > 0 && loadedPath !== asset.path) {
            player.stop();
            loadedPath = "";
            loadedAdjustmentKey = "";
        }
        Qt.callLater(refreshAsset);
    }

    onActiveChanged: {
        if (active) {
            Qt.callLater(refreshAsset);
        }
    }

    Connections {
        target: backend

        function onAssetsChanged(): void {
            if (preview.active) {
                preview.refreshAsset();
            }
        }

        function onJobsChanged(): void {
            if (preview.active && preview.hasAsset) {
                preview.analysisStatus = backend.analysisStatusForAsset(preview.asset.id);
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
        visible: !preview.hasAsset

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
            text: qsTr("The waveform, playback controls, and extracted text stay together in this workspace.")
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
        visible: preview.hasAsset

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3

                Text {
                    Layout.fillWidth: true
                    text: preview.hasAsset ? preview.fileName(preview.asset.path) : ""
                    color: Theme.textPrimary
                    font.pixelSize: 20
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: preview.hasAsset ? preview.asset.path : ""
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideMiddle
                }
            }

            Rectangle {
                Layout.preferredWidth: statusText.implicitWidth + 16
                Layout.preferredHeight: 24
                radius: 12
                color: preview.hasAsset && preview.asset.pathStatus === "missing" ? Theme.warningSurface : Theme.accentSurfaceQuiet

                Text {
                    id: statusText
                    anchors.centerIn: parent
                    text: preview.hasAsset && preview.asset.pathStatus === "missing" ? qsTr("Missing") : preview.hasAsset && Number(preview.asset.adjustmentRevision) > 0 ? qsTr("Saved version") : qsTr("Original preserved")
                    color: preview.hasAsset && preview.asset.pathStatus === "missing" ? Theme.warningText : Theme.accentSelectionText
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
            Layout.minimumHeight: 150
            Layout.preferredHeight: Math.max(170, Math.min(250, preview.height * 0.28))
            radius: 12
            color: Theme.waveformSurface
            border.color: Theme.borderStrong

            WaveformView {
                anchors.fill: parent
                anchors.margins: 18
                levels: preview.waveformLevels
                progress: preview.ownsActivePlayback() && player.duration > 0 ? player.position / player.duration : 0
            }

            MouseArea {
                anchors.fill: parent
                enabled: preview.hasAsset && player.duration > 0 && preview.ownsActivePlayback()
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: function (mouse) {
                    const target = Math.round(mouse.x / width * player.duration);
                    player.seek(Math.max(preview.trimStartMillis, Math.min(target, preview.trimEndMillis)));
                }
            }

            Text {
                anchors.centerIn: parent
                visible: preview.waveformLevels.length === 0
                text: preview.hasAsset && preview.asset.pathStatus === "missing" ? qsTr("Original file is unavailable") : qsTr("Preparing waveform…")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            EchoIconButton {
                source: preview.hasAsset && player.playing && preview.ownsActivePlayback() ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: player.playing ? qsTr("Pause") : qsTr("Play")
                enabled: preview.hasAsset && preview.asset.pathStatus !== "missing"
                buttonSize: 38
                iconSize: 19
                onClicked: {
                    if (!preview.ownsActivePlayback()) {
                        preview.playFrom(preview.trimStartMillis);
                    } else {
                        player.togglePause();
                    }
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/stop.svg"
                toolTipText: qsTr("Stop")
                enabled: preview.ownsActivePlayback()
                buttonSize: 34
                iconSize: 16
                onClicked: player.stop()
            }

            Text {
                text: preview.formatDuration(player.position) + " / " + preview.formatDuration(player.duration)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Slider {
                Layout.fillWidth: true
                from: preview.trimStartMillis
                to: Math.max(preview.trimStartMillis + 1, preview.trimEndMillis)
                value: player.duration > 0 ? player.position : 0
                enabled: preview.ownsActivePlayback() && player.duration > 0
                onMoved: player.seek(value)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Text")
                hint: preview.analysisHint()
            }

            BusyIndicator {
                visible: preview.analysisActive
                running: visible
                Layout.preferredWidth: 18
                Layout.preferredHeight: 18
            }

            EchoButton {
                visible: preview.analysisFailed && preview.analysisStatus.recovery === "manual"
                text: preview.analysisStatus.stage === "contextual" ? qsTr("Retry sound understanding") : preview.analysisStatus.stage === "alignment" ? qsTr("Retry timing analysis") : preview.analysisStatus.stage === "text" ? qsTr("Retry text extraction") : qsTr("Retry analysis")
                enabled: preview.hasAsset && preview.asset.pathStatus !== "missing"
                ghost: true
                onClicked: {
                    if (backend.retryAnalysis(preview.asset.id)) {
                        preview.analysisStatus = backend.analysisStatusForAsset(preview.asset.id);
                    }
                }
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
                text: preview.analysisStatus.recovery === "automatic" ? qsTr("Echo will resume when Infer Runtime is available.") : preview.analysisStatus.recovery === "source" ? qsTr("Reconnect the Original before analysis can continue.") : preview.analysisFailed ? qsTr("Background text extraction did not complete. You can retry this sound.") : preview.analysisStatus.stage === "sound_events" ? qsTr("No speech was detected. Echo is identifying audible events.") : preview.analysisStatus.stage === "complete" ? qsTr("No speech was detected in this sound.") : preview.analysisActive ? qsTr("Echo is extracting text from this sound through Infer Runtime.") : qsTr("Text is extracted automatically after import.")
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
                    color: player.duration > 0 && player.position >= modelData.start * 1000 && player.position <= modelData.end * 1000 ? Theme.accentSurface : Theme.transparent

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (!preview.ownsActivePlayback()) {
                                preview.playFrom(modelData.start * 1000);
                            } else {
                                player.seek(modelData.start * 1000);
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
