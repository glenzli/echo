//! Snapshot comparison and isolated audition. Restoring emits an authored draft request only.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "AssemblyVersionComparison.js" as Comparison
Popup {
    id: dialog
    required property var workspace
    required property var catalogBackend
    required property var renderer
    required property var transport
    property var current: ({})
    property var saved: null
    property var revisions: []
    property bool moreAvailable: false
    property string errorText: ""
    property string listeningSide: ""
    property string pendingSide: ""
    property real resumePosition: 0
    property real retainedRevisionId: 0
    readonly property var differences: saved ? Comparison.compare(current,saved) : []
    signal restoreRequested(var revision)
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(980,parent.width-32); height: Math.min(660,parent.height-32)
    padding: 20; modal: true; dim: true; focus: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { radius: 12; color: Theme.panelRaised; border.color: Theme.border }
    function present() {
        if (!workspace.hasDocument) return;
        workspace.stopPlayback(); materialPlayer.stop();
        current=JSON.parse(JSON.stringify(workspace.document)); saved=null; revisions=[];
        errorText=""; listeningSide=""; pendingSide=""; resumePosition=0;
        const retained=workspace.libraryAssets.find(asset=>asset.assemblyId===current.id);
        retainedRevisionId=retained ? Number(retained.assemblyRevisionId || 0) : 0;
        open(); loadMore();
        if (revisions.length) selectRevision(workspace.dirty ? revisions[0] : revisions.find(row=>Number(row.revisionId)!==Number(current.revisionId)) || revisions[0]);
    }
    function loadMore() {
        const page=catalogBackend.soundAssemblyHistory(current.id,revisions.length ? revisions[revisions.length-1].revisionNumber : 0);
        if (page.error) { errorText=page.error; return; }
        revisions=revisions.concat(Array.from(page.revisions || [])); moreAvailable=(page.revisions || []).length===50;
    }
    function stop() {
        pendingSide=""; listeningSide="";
        transport.stop(); renderer.cancel();
    }
    function selectRevision(row) {
        stop(); resumePosition=0; errorText="";
        const value=catalogBackend.soundAssemblyAtRevision(current.id,row.revisionId);
        saved=value.error ? null : value;
        if (value.error) errorText=value.error;
    }
    function listen(side) {
        if (renderer.running) return;
        if (listeningSide===side && transport.active) { transport.togglePause(); return; }
        if (transport.active) resumePosition=transport.position;
        transport.stop(); listeningSide=""; pendingSide=side; errorText="";
        const target=side==="current" ? current : saved;
        if (resumePosition>=workspace.assemblyDuration(target)) {
            pendingSide=""; errorText=qsTr("This version ends before the current position. Stop to compare from the start."); return;
        }
        renderer.preparePreview(target);
    }
    function rowTitle(row) {
        if (row.kind==="master") return qsTr("Master output");
        if (row.kind==="name") return qsTr("Project name");
        if (row.kind==="clip") {
            const asset=workspace.libraryAssets.find(value=>value.id===row.assetId);
            return asset ? SoundSemantics.sourceTitle(asset) : qsTr("Unavailable source");
        }
        return row.name || (row.kind==="track" ? qsTr("Track") : qsTr("Marker"));
    }
    function changeLabel(row) {
        const state=row.change==="added"?qsTr("Added"):row.change==="removed"?qsTr("Removed"):qsTr("Changed");
        const labels={position:qsTr("Position"),range:qsTr("Source range"),source:qsTr("Source"),processing:qsTr("Processing version"),mix:qsTr("Mix"),fades:qsTr("Fades"),envelope:qsTr("Gain envelope")};
        return state+((row.groups || []).length ? " · "+row.groups.map(value=>labels[value]).join(" · ") : "");
    }
    function restore() { if (saved && !renderer.running && differences.length) { const value=saved; close(); restoreRequested(value); } }
    onClosed: stop()
    Connections {
        target: dialog.renderer
        function onStateChanged() {
            if (!dialog.visible || dialog.renderer.running || !dialog.pendingSide) return;
            const side=dialog.pendingSide; dialog.pendingSide="";
            if (dialog.renderer.errorText || !dialog.renderer.hasPreview) { dialog.errorText=dialog.renderer.errorText; return; }
            if (dialog.renderer.playPreview(Math.round(dialog.resumePosition))) dialog.listeningSide=side;
            else dialog.errorText=dialog.renderer.errorText;
        }
    }
    contentItem: ColumnLayout {
        spacing: 14
        Text { text: qsTr("Compare project versions"); font.pixelSize: 20; font.weight: Font.DemiBold; color: Theme.textPrimary }
        Text { Layout.fillWidth: true; text: qsTr("Compare a saved version with your current draft. Listening here leaves both the draft and the retained listening version unchanged."); wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 16
            ColumnLayout {
                Layout.preferredWidth: 280; Layout.maximumWidth: 280; Layout.fillHeight: true; spacing: 8
                Text { text: qsTr("Saved versions"); color: Theme.textSecondary; font.pixelSize: Theme.fontSection }
                ListView {
                    id: versionList; objectName: "assemblyVersionList"
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 6; model: dialog.revisions
                    ScrollBar.vertical: ScrollBar {}
                    delegate: ItemDelegate {
                        required property var modelData
                        width: versionList.width; height: 72
                        onClicked: dialog.selectRevision(modelData)
                        background: Rectangle { radius: 7; color: dialog.saved && dialog.saved.revisionId===modelData.revisionId ? Theme.surfaceSelected : Theme.surfaceSubtle; border.color: dialog.saved && dialog.saved.revisionId===modelData.revisionId ? Theme.accentBorder : Theme.border }
                        contentItem: ColumnLayout {
                            spacing: 3
                            Text { text: qsTr("Version %1").arg(modelData.revisionNumber)+(modelData.revisionId===dialog.retainedRevisionId ? " · "+qsTr("Retained") : ""); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.Medium }
                            Text { text: new Date(modelData.updatedAtMillis).toLocaleString(Qt.locale(),Locale.ShortFormat); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                            Text { text: qsTr("%1 tracks · %2 clips").arg(modelData.trackCount).arg(modelData.clipCount); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
                        }
                    }
                }
                EchoButton { visible: dialog.moreAvailable; text: qsTr("Load earlier versions"); ghost: true; onClicked: dialog.loadMore() }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 12
                Text { Layout.fillWidth: true; text: dialog.saved ? qsTr("Current draft compared with version %1").arg(dialog.saved.revisionNumber) : qsTr("Choose a saved version"); wrapMode: Text.WordWrap; color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.Medium }
                RowLayout {
                    Layout.fillWidth: true
                    EchoButton { objectName: "historyListenCurrent"; Layout.fillWidth: true; text: qsTr("A · Current draft"); selected: dialog.listeningSide==="current"; ghost: true; enabled: !!dialog.saved && !dialog.renderer.running; onClicked: dialog.listen("current") }
                    EchoButton { objectName: "historyListenSaved"; Layout.fillWidth: true; text: qsTr("B · Saved version"); selected: dialog.listeningSide==="saved"; ghost: true; enabled: !!dialog.saved && !dialog.renderer.running; onClicked: dialog.listen("saved") }
                    EchoIconButton { source: "qrc:/EchoDesktop/icons/stop.svg"; toolTipText: qsTr("Stop comparison"); onClicked: { dialog.stop(); dialog.resumePosition=0; } }
                }
                Text { Layout.fillWidth: true; text: dialog.renderer.running ? qsTr("Preparing comparison…") : qsTr("Switching A/B continues at the same time position."); color: Theme.textMuted; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
                ListView {
                    id: changeList; objectName: "assemblyVersionChanges"
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 6; model: dialog.differences
                    ScrollBar.vertical: ScrollBar {}
                    Text { anchors.centerIn: parent; visible: !!dialog.saved && dialog.differences.length===0; text: qsTr("No authored differences"); color: Theme.textMuted; font.pixelSize: Theme.fontBody }
                    delegate: Rectangle {
                        required property var modelData
                        width: changeList.width; height: changeDetails.implicitHeight+18; radius: 7; color: Theme.surfaceSubtle
                        ColumnLayout {
                            id: changeDetails; anchors.fill: parent; anchors.margins: 9; spacing: 4
                            Text { Layout.fillWidth: true; text: dialog.rowTitle(modelData); elide: Text.ElideRight; font.pixelSize: Theme.fontBody; color: Theme.textPrimary }
                            Text { Layout.fillWidth: true; text: dialog.changeLabel(modelData); wrapMode: Text.WordWrap; font.pixelSize: Theme.fontMeta; color: Theme.textSecondary }
                        }
                    }
                }
            }
        }
        Text { Layout.fillWidth: true; visible: !!dialog.errorText; text: dialog.errorText; wrapMode: Text.WordWrap; color: Theme.warningText; font.pixelSize: Theme.fontBody }
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: qsTr("Restoring creates an undoable draft. Keep it in memories separately when ready."); wrapMode: Text.WordWrap; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            EchoButton { text: qsTr("Close"); ghost: true; onClicked: dialog.close() }
            EchoButton { objectName: "historyRestore"; text: qsTr("Restore as draft"); enabled: !!dialog.saved && dialog.differences.length>0 && !dialog.renderer.running; onClicked: dialog.restore() }
        }
    }
}
