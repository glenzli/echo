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
    palette.window: Theme.window
    palette.windowText: Theme.textPrimary
    palette.base: Theme.control
    palette.alternateBase: Theme.panel
    palette.text: Theme.textPrimary
    palette.button: Theme.buttonSurface
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.accentText
    palette.placeholderText: Theme.textDisabled
    palette.mid: Theme.border
    palette.dark: Theme.borderStrong
    palette.light: Theme.surfaceSubtle

    readonly property string memorySmokeReport: independentDisclosureValidation ? disclosureSmoke.reportJson : memorySmoke.reportJson
    SourceDisclosureSmoke {
        id: disclosureSmoke; shell: window; editor: soundEditor; library: audioSpace
        fixtureRoot: independentDisclosureValidation ? independentSmokeRoot : ""; reopening: false
    }
    readonly property int memorySmokeStage: independentDisclosureValidation ? disclosureSmoke.stage : memorySmoke.stage
    MemoryWorkflowSmoke {
        id: memorySmoke
        shell: window
        assembly: soundAssembly
        editor: soundEditor
        library: audioSpace
        materials: materialsLibrary
        materialPath: independentDisclosureValidation ? "" : memorySmokeMaterial
    }

    readonly property alias multitrackSmokeReport: multitrackSmoke.reportJson
    readonly property alias multitrackSmokeStage: multitrackSmoke.stage
    MultitrackWorkflowSmoke {
        id: multitrackSmoke
        shell: window
        assembly: soundAssembly
        fixtureRoot: multitrackSmokeRoot
        stress: complexSmokeEnabled
    }

    readonly property alias spectralSmokeReport: spectralSmoke.reportJson
    readonly property alias spectralSmokeStage: spectralSmoke.stage
    SpectralWorkflowSmoke {
        id: spectralSmoke
        shell: window; editor: soundEditor; library: audioSpace
        fixtureRoot: spectralSmokeRoot
    }

    readonly property alias noiseSmokeReport: noiseSmoke.reportJson
    readonly property alias noiseSmokeStage: noiseSmoke.stage
    NoiseWorkflowSmoke {
        id: noiseSmoke
        shell: window; editor: soundEditor; library: audioSpace
        fixtureRoot: noiseSmokeRoot
    }

    property var jobSnapshot: ({
            pending: 0,
            running: 0,
            done: 0,
            failed: 0
        })
    property bool jobsWereActive: false
    property string jobSignature: ""
    property int workspaceIndex: 0
    property var projectEditAsset: null
    property string projectClipId: ""

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

    function debugOpenSpectralSource(path: string): void {
        const asset=backend.listAssets().find(value=>value.path===path);
        if(!asset) return;
        audioSpace.selectedFilter="all";audioSpace.refreshAssets();audioSpace.selectAssetOnly(asset);
        showSoundEditor();soundEditor.spectralFocus=true;
        Qt.callLater(()=>{
            const regions=soundEditor.adjustment.spectralRepair.regions;
            soundEditor.spectralEditor.noiseTools=!!soundEditor.adjustment.spectralRepair.noiseProfile;
            if(regions.length) soundEditor.spectralEditor.setSelection(regions[0],0);
            soundEditor.spectralEditor.lowHertz=200;soundEditor.spectralEditor.highHertz=8000;
        });
    }

    function showSoundEditor(): void {
        if (audioSpace.selectedAsset !== null) {
            if (audioSpace.selectedAsset.assemblyId) {
                showSoundAssembly();
                soundAssembly.openAssembly(audioSpace.selectedAsset.assemblyId);
                return;
            }
            projectClipId = "";
            soundEditor.projectDocument = ({});
            workspaceIndex = 1;
        }
    }

    function showSoundAssembly(): void {
        workspaceIndex = 3;
        materialPlayer.stop();
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

    function debugOpenSoundTape(): void {
        showAudioSpace();
        audioSpace.debugOpenSoundTape();
    }

    function debugPlaySoundTape(): void {
        showAudioSpace();
        audioSpace.debugPlaySoundTape();
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
        onMaterialsRequested: window.workspaceIndex = 4
        onSettingsRequested: window.openSettings()
    }

    Shortcut { sequence: "Ctrl+Shift+E"; onActivated: independentEditor.launchEditor([]) }

    StackLayout {
        anchors.fill: parent
        currentIndex: window.workspaceIndex

        AudioSpaceWorkspace {
            id: audioSpace

            Layout.fillWidth: true
            Layout.fillHeight: true
            jobStats: window.jobSnapshot
            onOpenLibraryRequested: window.openLibrary()
            onOpenProjectRequested: id => { window.showSoundAssembly(); soundAssembly.openAssembly(id); }
        }

        SoundEditingWorkspace {
            id: soundEditor

            Layout.fillWidth: true
            Layout.fillHeight: true
            asset: window.projectClipId ? window.projectEditAsset : audioSpace.selectedAsset
            projectClipId: window.projectClipId
            onReturnToProjectRequested: {
                if (dirty) save();
                if (!dirty) window.showSoundAssembly();
            }
            onProjectClipSaved: revision => soundAssembly.acceptClipRevision(revision)
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
            onEditClipRequested: function (asset, revision, clipId) {
                window.projectEditAsset = asset;
                window.projectClipId = clipId;
                soundEditor.projectDocument = revision;
                window.workspaceIndex = 1;
            }
        }
        SoundSourceBrowser {
            id: materialsLibrary
            Layout.fillWidth: true
            Layout.fillHeight: true
            onOpenAssemblyRequested: id => { window.showSoundAssembly(); soundAssembly.openAssembly(id); }
        }
    }

    Connections {
        target: audioSpace

        function onSelectedAssetChanged(): void {
            if (window.workspaceIndex === 1 && !window.projectClipId && audioSpace.selectedAsset === null) {
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
