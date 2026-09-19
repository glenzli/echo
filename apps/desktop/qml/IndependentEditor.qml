//! A document-oriented shell over the shared single-source and multitrack editors.
//! It never constructs Library browsing, scanning, or automatic analysis surfaces.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import EchoDesktop

ApplicationWindow {
    id: window
    width: 1500; height: 900
    minimumWidth: 1240; minimumHeight: 720
    visible: true
    color: Theme.window
    title: (independentEditor.projectPath ? independentEditor.projectPath.split("/").pop() : qsTr("Untitled project")) + (projectDirty ? " *" : "") + " — " + qsTr("Echo · Independent editing")
    palette.window: Theme.window
    palette.windowText: Theme.textPrimary
    palette.text: Theme.textPrimary
    palette.base: Theme.control
    palette.button: Theme.buttonSurface
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.accentText
    readonly property alias independentSmokeReport: smoke.reportJson
    readonly property alias independentSmokeStage: smoke.stage
    IndependentEditorSmoke { id: smoke; shell: window; editor: editor; assembly: assembly; fixtureRoot: independentSmokeRoot; reopening: independentSmokeReopen }
    property var assets: []
    property var selectedAsset: null
    property var clipAsset: null
    property string clipId: ""
    property bool multitrack: false
    property bool projectDirty: false
    property bool ready: false
    property bool allowClose: false
    property bool closeAfterSave: false
    property string notice: ""
    readonly property bool processing: independentEditor.busy || renderExporter.running || renderedSpectralWorkingCopy.running || soundAssemblyController.running || noiseProfile.running

    function refreshSources(): void {
        const selectedId = selectedAsset ? selectedAsset.id : "";
        assets = backend.listAssets().filter(value => !value.assemblyId);
        selectedAsset = assets.find(value => value.id === selectedId) || assets[0] || null;
    }
    function flushDrafts(): bool {
        if (editor.dirty) editor.save();
        if (editor.dirty) return false;
        if (assembly.dirty && !assembly.saveRevision()) return false;
        return true;
    }
    function chooseSource(index: int): void {
        if (!flushDrafts()) return;
        clipId = ""; clipAsset = null;
        selectedAsset = assets[index] || null;
        multitrack = false;
    }
    function showMultitrack(): void {
        if (!flushDrafts()) return;
        clipId = ""; clipAsset = null;
        if (!assembly.hasDocument && assets.length) {
            assembly.loadRevision(backend.createSoundAssembly(qsTr("Untitled project"), assets.map(value => value.id), assets.length <= 8 ? "layered" : "sequence"));
            projectDirty = true;
        }
        multitrack = true;
        assembly.sourcesVisible = true;
    }
    function saveProject(saveAs: bool): void {
        if (processing || !flushDrafts()) return;
        player.stop(); materialPlayer.stop();
        if (saveAs || !independentEditor.projectPath) saveDialog.open();
        else independentEditor.saveProject(independentEditor.projectUrl);
    }
    function exportAudio(): void {
        if (processing) return;
        if (multitrack) assembly.exportMix(); else editor.openExport();
    }
    function finishClose(): void {
        independentEditor.finishSession(); allowClose = true; window.close();
    }
    Component.onCompleted: {
        refreshSources();
        assembly.refreshAssemblies();
        if (assembly.hasDocument) multitrack = true;
        ready = true; projectDirty = assets.length > 0 && !independentEditor.projectPath;
    }
    onClosing: close => {
        if (allowClose) return;
        close.accepted = false;
        if (processing) { notice = qsTr("Finish or cancel the current operation before closing."); return; }
        if (projectDirty || editor.dirty || assembly.dirty) closeDialog.open();
        else finishClose();
    }
    Connections {
        target: backend
        function onAssetsChanged(): void { refreshSources(); if (ready) projectDirty = true; }
        function onProcessingRecipesChanged(): void { if (ready) projectDirty = true; }
    }
    Connections {
        target: independentEditor
        function onAudioImported(ids): void {
            backend.refresh(); refreshSources();
            if (ids.length) selectedAsset = assets.find(value => value.id === ids[0]) || selectedAsset;
            if (multitrack && !assembly.hasDocument) showMultitrack();
            projectDirty = true;
        }
        function onProjectSaved(): void {
            projectDirty = false;
            if (closeAfterSave) { closeAfterSave = false; finishClose(); }
        }
    }
    Timer {
        id: recoveryTimer; interval: 1500
        onTriggered: if (!processing && !editor.adjustment.gestureActive) flushDrafts(); else restart();
    }
    Shortcut { sequences: [StandardKey.Save]; onActivated: saveProject(false) }
    Shortcut { sequences: [StandardKey.SaveAs]; onActivated: saveProject(true) }
    Shortcut { sequences: [StandardKey.Open]; onActivated: audioDialog.open() }
    Shortcut { sequences: [StandardKey.New]; onActivated: independentEditor.launchEditor([]) }

    menuBar: MenuBar {
        Menu {
            title: qsTr("File")
            MenuItem { text: qsTr("New editing window"); onTriggered: independentEditor.launchEditor([]) }
            MenuItem { text: qsTr("Open audio…"); enabled: !window.processing; onTriggered: audioDialog.open() }
            MenuItem { text: qsTr("Open project…"); onTriggered: projectDialog.open() }
            MenuSeparator {}
            MenuItem { text: qsTr("Save project"); enabled: !window.processing; onTriggered: window.saveProject(false) }
            MenuItem { text: qsTr("Save project as…"); enabled: !window.processing; onTriggered: window.saveProject(true) }
            MenuItem { text: qsTr("Export audio…"); enabled: window.selectedAsset !== null && !window.processing; onTriggered: window.exportAudio() }
            MenuSeparator {}
            MenuItem { text: qsTr("Close window"); onTriggered: window.close() }
        }
    }
    header: ToolBar {
        background: Rectangle { color: Theme.chrome; border.color: Theme.border }
        contentItem: RowLayout {
            spacing: 12
            Text { Layout.leftMargin: 16; text: "ECHO"; font.letterSpacing: 2; font.weight: Font.DemiBold; color: Theme.textPrimary }
            Text { text: qsTr("Independent editing"); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoButton { text: qsTr("Waveform / Spectrum"); ghost: window.multitrack; enabled: window.selectedAsset !== null && !window.processing; onClicked: { if (window.flushDrafts()) { window.clipId = ""; window.multitrack = false; } } }
            EchoButton { text: qsTr("Multitrack"); ghost: !window.multitrack; enabled: !window.processing; onClicked: window.showMultitrack() }
            Item { Layout.fillWidth: true }
            EchoButton { text: qsTr("Open audio…"); enabled: !window.processing; onClicked: audioDialog.open() }
            EchoButton { text: qsTr("Save project"); enabled: !window.processing; onClicked: window.saveProject(false) }
            EchoButton { Layout.rightMargin: 12; text: qsTr("Export audio…"); enabled: window.selectedAsset !== null && !window.processing; onClicked: window.exportAudio() }
        }
    }
    ColumnLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            visible: !window.multitrack && window.assets.length > 0 && !window.clipId
            Layout.fillWidth: true; implicitHeight: 42; color: Theme.panel
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 16; anchors.rightMargin: 16
                Text { text: qsTr("Project sources"); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                EchoComboBox { Layout.preferredWidth: 400; model: window.assets.map(value => SoundSemantics.sourceTitle(value)); selectionIndex: window.assets.findIndex(value => window.selectedAsset && value.id === window.selectedAsset.id); enabled: !window.processing; onActivated: window.chooseSource(currentIndex) }
                Item { Layout.fillWidth: true }
                Text { text: qsTr("Original files stay unchanged"); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            }
        }
        StackLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            enabled: !independentEditor.busy
            currentIndex: window.assets.length === 0 ? 0 : window.multitrack ? 2 : 1
            Item {
                ColumnLayout {
                    anchors.centerIn: parent; width: 480; spacing: 18
                    Text { Layout.fillWidth: true; text: qsTr("A space for the sound in front of you"); font.pixelSize: 26; font.weight: Font.DemiBold; color: Theme.textPrimary; wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter }
                    Text { Layout.fillWidth: true; text: qsTr("Drop audio here to edit, repair or arrange it. Save a project to continue later, or export your finished sound."); font.pixelSize: Theme.fontBody; color: Theme.textMuted; wrapMode: Text.WordWrap; horizontalAlignment: Text.AlignHCenter }
                    RowLayout {
                        Layout.alignment: Qt.AlignHCenter
                        EchoButton { text: qsTr("Open audio…"); onClicked: audioDialog.open() }
                        EchoButton { text: qsTr("Open project…"); ghost: true; onClicked: projectDialog.open() }
                    }
                    Repeater {
                        model: independentEditor.recoverableSessions
                        EchoButton { required property var modelData; Layout.alignment: Qt.AlignHCenter; text: qsTr("Recover editing session · %1").arg(modelData.name); ghost: true; onClicked: independentEditor.resumeSession(modelData.path) }
                    }
                }
            }
            SoundEditingWorkspace {
                id: editor
                asset: window.clipId ? window.clipAsset : window.selectedAsset
                projectClipId: window.clipId
                onDirtyChanged: if (dirty) { window.projectDirty = true; recoveryTimer.restart(); }
                onReturnToProjectRequested: { if (window.flushDrafts()) { window.clipId = ""; window.multitrack = true; } }
                onProjectClipSaved: revision => assembly.acceptClipRevision(revision)
            }
            SoundAssemblyWorkspace {
                id: assembly
                onDirtyChanged: if (dirty) { window.projectDirty = true; recoveryTimer.restart(); }
                onEditClipRequested: function(asset, revision, clipId) {
                    if (!window.flushDrafts()) return;
                    window.clipAsset = asset; window.clipId = clipId; editor.projectDocument = revision; window.multitrack = false;
                }
            }
        }
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: statusText.implicitHeight + 16
            color: Theme.chrome
            RowLayout {
                anchors.fill: parent; anchors.margins: 8
                BusyIndicator { visible: independentEditor.busy; running: visible; implicitWidth: 22; implicitHeight: 22 }
                Text { id: statusText; Layout.fillWidth: true; text: independentEditor.errorText || window.notice || (independentEditor.busy ? qsTr("Working on your project…") : window.projectDirty ? qsTr("Changes are recoverable. Save the project to keep a portable copy.") : qsTr("Only this project is open. Automatic library analysis is off.")); wrapMode: Text.WordWrap; color: independentEditor.errorText ? Theme.warningText : Theme.textMuted; font.pixelSize: Theme.fontMeta }
            }
        }
    }
    DropArea {
        anchors.fill: parent
        enabled: !window.processing
        onDropped: drop => {
            if (!drop.hasUrls) return;
            if (!window.flushDrafts()) return;
            independentEditor.importAudio(drop.urls); drop.acceptProposedAction();
        }
    }
    FileDialog {
        id: audioDialog; title: qsTr("Open audio"); fileMode: FileDialog.OpenFiles
        nameFilters: [qsTr("Audio files (*.wav *.mp3 *.m4a *.aac *.flac *.ogg *.aiff *.aif *.caf)"), qsTr("All files (*)")]
        onAccepted: if (window.flushDrafts()) independentEditor.importAudio(selectedFiles)
    }
    FileDialog {
        id: projectDialog; title: qsTr("Open project"); fileMode: FileDialog.OpenFile
        nameFilters: [qsTr("Echo projects (*.echo)")]
        onAccepted: independentEditor.launchProject(selectedFile)
    }
    FileDialog {
        id: saveDialog; title: qsTr("Save project"); fileMode: FileDialog.SaveFile
        nameFilters: [qsTr("Echo projects (*.echo)")]; defaultSuffix: "echo"
        onAccepted: independentEditor.saveProject(selectedFile)
        onRejected: window.closeAfterSave = false
    }
    Dialog {
        id: closeDialog; anchors.centerIn: Overlay.overlay; modal: true; width: 460
        title: qsTr("Save this project before closing?")
        contentItem: Text { text: qsTr("The saved project includes your audio sources and editing state."); color: Theme.textPrimary; wrapMode: Text.WordWrap }
        footer: RowLayout {
            EchoButton { text: qsTr("Cancel"); ghost: true; onClicked: closeDialog.close() }
            EchoButton { text: qsTr("Discard changes"); ghost: true; onClicked: { closeDialog.close(); window.finishClose(); } }
            EchoButton { text: qsTr("Save project"); onClicked: { closeDialog.close(); window.closeAfterSave = true; window.saveProject(false); } }
        }
    }
}
