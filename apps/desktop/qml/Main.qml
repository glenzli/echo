//! Echo application shell. Workspace chrome, page routing, and process-level
//! lifecycle live here; Audio Space, Sound Adjustments, Sound Assembly, and Audio Library own
//! their presentation and interaction internally.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

ApplicationWindow {
    id: window

    width: 1500
    height: 900
    minimumWidth: 1240
    minimumHeight: 720
    visible: true
    title: Qt.platform.os === "osx" ? "" : qsTr("Echo")
    flags: Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint
    color: Theme.window

    property var jobSnapshot: ({
            pending: 0,
            running: 0,
            done: 0,
            failed: 0
        })
    property bool jobsWereActive: false
    property string jobSignature: ""
    property int workspaceIndex: 0

    readonly property bool jobsActive: jobSnapshot.pending > 0 || jobSnapshot.running > 0

    function openSettings(): void {
        settingsDialog.open();
    }

    function openLibrary(): void {
        workspaceIndex = 2;
        audioLibrary.refresh();
    }

    function showAudioSpace(): void {
        workspaceIndex = 0;
    }

    function showSoundEditor(): void {
        if (audioSpace.selectedAsset !== null) {
            workspaceIndex = 1;
        }
    }

    function showSoundAssembly(): void {
        workspaceIndex = 3;
        soundAssembly.refreshAssemblies();
    }

    function createSoundAssembly(assetIds: var, layout: string): void {
        if (!assetIds || assetIds.length === 0)
            return;
        const defaultName = layout === "layered"
            ? qsTr("Layered assembly") : qsTr("Sound sequence");
        const created = backend.createSoundAssembly(defaultName, assetIds, layout);
        if (created.error) {
            soundAssembly.presentError(created.error);
            showSoundAssembly();
            return;
        }
        showSoundAssembly();
        soundAssembly.loadRevision(created);
    }

    function debugReplaySoundEditor(): void {
        if (audioSpace.selectedAsset === null)
            return;
        showSoundEditor();
        soundEditor.togglePlayback();
        debugEqualizerTimer.start();
        debugReplayTimer.start();
        debugAnalysisTimer.start();
    }

    function debugCreateSoundAssembly(): void {
        if (audioSpace.selectedAsset === null)
            return;
        createSoundAssembly([audioSpace.selectedAsset.id], "sequence");
    }

    function debugExportSoundAssembly(destination: url): void {
        if (!soundAssembly.hasDocument)
            return;
        soundAssemblyController.exportAssembly(soundAssembly.document, destination);
    }

    function debugOpenExportDialog(): void {
        if (audioSpace.selectedAsset === null)
            return;
        showSoundEditor();
        soundEditor.openExport();
    }

    function debugOpenAlbumDialog(): void {
        showAudioSpace();
        audioSpace.debugOpenNewAlbumDialog();
    }

    function debugCreateAlbum(name: string): void {
        showAudioSpace();
        audioSpace.debugCreateAlbum(name);
    }

    function debugSearch(text: string): void {
        showAudioSpace();
        audioSpace.setSearchText(text);
    }

    function debugExportSound(destination: url): void {
        if (audioSpace.selectedAsset === null)
            return;
        showSoundEditor();
        soundEditor.debugExport(destination);
    }

    function debugOpenBatchDialog(): void {
        showAudioSpace();
        audioSpace.debugOpenBatchDialog();
    }

    function debugBatchExport(destination: url, format: string): void {
        showAudioSpace();
        audioSpace.debugBatchExport(destination, format);
    }

    onWorkspaceIndexChanged: {
        player.stop();
        if (workspaceIndex !== 3 && soundAssemblyController.running)
            soundAssemblyController.cancel();
    }

    EchoSettingsDialog {
        id: settingsDialog
    }

    header: MainTitleBar {
        hostWindow: window
        editor: soundEditor
        assembly: soundAssembly
        workspaceIndex: window.workspaceIndex
        editorAvailable: audioSpace.selectedAsset !== null
        jobsActive: window.jobsActive
        activeJobCount: window.jobSnapshot.pending + window.jobSnapshot.running
        onSoundWallRequested: window.showAudioSpace()
        onSoundEditorRequested: window.showSoundEditor()
        onSoundAssemblyRequested: window.showSoundAssembly()
        onSettingsRequested: window.openSettings()
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: window.workspaceIndex

        AudioSpaceWorkspace {
            id: audioSpace

            Layout.fillWidth: true
            Layout.fillHeight: true
            jobStats: window.jobSnapshot
            onOpenLibraryRequested: window.openLibrary()
        }

        SoundEditingWorkspace {
            id: soundEditor

            Layout.fillWidth: true
            Layout.fillHeight: true
            asset: audioSpace.selectedAsset
        }

        AudioLibraryWorkspace {
            id: audioLibrary

            Layout.fillWidth: true
            Layout.fillHeight: true
            jobStats: window.jobSnapshot
            onCloseRequested: window.showAudioSpace()
        }


        SoundAssemblyWorkspace {
            id: soundAssembly

            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    Connections {
        target: audioSpace

        function onSelectedAssetChanged(): void {
            if (window.workspaceIndex === 1 && audioSpace.selectedAsset === null) {
                window.showAudioSpace();
            }
        }
    }

    Connections {
        target: audioSpace

        function onAssemblyRequested(assetIds, layout): void {
            window.createSoundAssembly(assetIds, layout);
        }
    }

    Component.onCompleted: {
        settingsDialog.uiPrefs = uiPrefs;
        settingsDialog.inferencePrefs = inferencePrefs;
        Theme.mode = uiPrefs.mode;
        uiPrefs.modeChanged.connect(() => {
            Theme.mode = uiPrefs.mode;
        });
        backend.queueScans();
        audioSpace.refreshAssets();
        jobTimer.start();
    }

    Timer {
        id: jobTimer

        interval: 750
        repeat: true
        onTriggered: {
            const snapshot = backend.jobStats();
            const active = snapshot.pending > 0 || snapshot.running > 0;
            const signature = snapshot.pending + ":" + snapshot.running + ":" + snapshot.done + ":" + snapshot.failed;
            window.jobSnapshot = snapshot;
            if (signature !== window.jobSignature) {
                audioSpace.refreshAnalysisStatuses();
                window.jobSignature = signature;
            }
            if (window.jobsWereActive && !active) {
                backend.refresh();
            }
            window.jobsWereActive = active;
        }
    }

    Timer {
        id: debugEqualizerTimer

        interval: 150
        onTriggered: {
            soundEditor.debugNudgeEqualizer();
            soundEditor.debugEnableCompressor();
        }
    }

    Timer {
        id: debugReplayTimer

        interval: 350
        onTriggered: {
            player.stop();
            soundEditor.togglePlayback();
        }
    }

    Timer {
        id: debugAnalysisTimer

        interval: 450
        onTriggered: soundEditor.debugAnalyzeOutput()
    }
}
