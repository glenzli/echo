import QtQuick
import QtTest
import EchoDesktop
import "../../qml/AssemblyVersionComparison.js" as Comparison
TestCase {
    id: test; name: "AssemblyHistory"; width: 1100; height: 760; visible: true; when: windowShown
    property var restored: null
    property bool failRead: false
    property var savedFixture: ({})
    QtObject {
        id: workspace
        property bool hasDocument: true; property bool dirty: true
        property var document: ({})
        property var libraryAssets: [{id:"source",path:"/voice.wav",sourceTitle:"Voice"}]
        function stopPlayback() {}
        function assemblyDuration(value) { return 10000; }
    }
    QtObject { id: materialPlayer; function stop() {} }
    QtObject {
        id: catalog
        function soundAssemblyHistory(id,cursor) { return {revisions:[{revisionId:1,revisionNumber:1,updatedAtMillis:1,trackCount:1,clipCount:1}]}; }
        function soundAssemblyAtRevision(id,revision) { return test.failRead ? {error:"Unavailable"} : test.savedFixture; }
    }
    QtObject {
        id: transport
        property bool active: false; property bool playing: false; property real position: 0
        function stop() { active=false; playing=false; position=0; }
        function togglePause() { playing=!playing; }
    }
    QtObject {
        id: renderer
        property bool running: false; property bool hasPreview: false; property string errorText: ""
        property var prepared: null
        signal stateChanged()
        function preparePreview(value) { prepared=value; running=true; }
        function cancel() { running=false; hasPreview=false; stateChanged(); }
        function playPreview(position) { transport.active=true; transport.playing=true; transport.position=position; return true; }
        function complete() { running=false; hasPreview=true; stateChanged(); }
    }
    AssemblyHistoryDialog { id: dialog; workspace: workspace; catalogBackend: catalog; renderer: renderer; transport: transport; onRestoreRequested: revision=>test.restored=revision }
    function document() { return {id:"project",revisionId:1,revisionNumber:1,name:"Project",master:{gainCentibels:0},markers:[],tracks:[{id:"track",name:"Track",gainCentibels:0,clips:[{id:"clip",assetId:"source",timelineStartMillis:0,sourceStartMillis:0,sourceEndMillis:10000,gainCentibels:0}]}]}; }
    function init() {
        dialog.close(); tryCompare(dialog,"visible",false); restored=null; failRead=false;
        savedFixture=document(); workspace.document=document(); workspace.document.tracks[0].clips[0].gainCentibels=-300;
        renderer.errorText=""; transport.stop();
    }
    function openDialog() { dialog.present(); tryCompare(dialog,"opened",true); waitForRendering(dialog.contentItem); }
    function test_differences_are_authored_and_current_relative_to_saved() {
        const saved=document(), current=document();
        current.revisionId=99; current.clipSources=[{path:"changed-cache"}];
        compare(Comparison.compare(current,saved).length,0);
        current.tracks[0].clips[0].timelineStartMillis=100;
        current.tracks[0].clips[0].gainCentibels=-300;
        const rows=Comparison.compare(current,saved); compare(rows.length,1); compare(rows[0].groups,["position","mix"]);
        current.tracks[0].clips=[]; compare(Comparison.compare(current,saved)[0].change,"removed");
    }
    function test_audition_switch_keeps_position_and_does_not_mutate_draft() {
        const before=JSON.stringify(workspace.document); openDialog();
        mouseClick(findChild(dialog,"historyListenCurrent")); verify(renderer.running); renderer.complete();
        compare(dialog.listeningSide,"current"); transport.position=1800;
        mouseClick(findChild(dialog,"historyListenSaved")); renderer.complete();
        compare(dialog.listeningSide,"saved"); compare(transport.position,1800);
        compare(JSON.stringify(workspace.document),before); compare(restored,null);
        dialog.close(); verify(!transport.active);
    }
    function test_restore_is_explicit_and_unavailable_revision_cannot_restore() {
        openDialog(); compare(restored,null); compare(dialog.differences.length,1);
        mouseClick(findChild(dialog,"historyRestore")); compare(restored.revisionId,1);
        tryCompare(dialog,"visible",false); failRead=true; restored=null; openDialog();
        verify(!findChild(dialog,"historyRestore").enabled); compare(dialog.errorText,"Unavailable");
    }
    function test_late_preview_after_close_never_plays() {
        openDialog(); dialog.listen("current"); dialog.close(); renderer.complete();
        verify(!transport.active); compare(dialog.pendingSide,"");
    }
}
