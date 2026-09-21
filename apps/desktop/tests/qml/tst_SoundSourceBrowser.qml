import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test
    name: "SoundSourceBrowser"; width: 720; height: 720; visible: true; when: windowShown
    property var sources: []
    property var played: []
    QtObject {
        id: backend
        property bool independentEditing: false
        signal assetsChanged()
        function listAssets() { return test.sources; }
        function projectMaterials(id) { return ["a", "b"]; }
    }
    QtObject { id: player; property bool playing: false; function togglePause() { playing=false; } }
    QtObject {
        id: materialPlayer
        property bool active: false; property bool playing: false
        function stop() { active=false; playing=false; }
        function togglePause() { playing=!playing; }
        function playAdjusted(path) { test.played.push(path); active=true; playing=true; }
    }
    QtObject {
        id: materialSearch
        property var results: []; property string resultsQuery: ""; property bool running: false; property string errorText: ""
        function clear() { results=[]; resultsQuery=""; }
        function request(query) { resultsQuery=query; }
    }
    QtObject { id: generatedNarration
        property bool running: false; property bool accepting: false
        property string detailsJson: ""; property string errorText: ""; property url audioUrl: ""
        signal accepted(string assetId)
        function discard() {}
    }
    SoundSourceBrowser { id: browser; anchors.fill: parent }
    function source(id, generated) {
        return {id:id,path:"/test/"+id+".wav",pathStatus:"present",durationMillis:1000,inMaterials:true,inMemory:false,
            sourceTitle:id,adjustmentRevision:0,materialCategory:"voice",eventType:"",hasGeneratedSource:generated,
            sourceDisclosure:{sources:generated ? [{assetId:id,revisionId:0,spans:[{kind:"ai_generated",startMillis:0,endMillis:1000,note:""}]}] : []}};
    }
    function init() {
        materialPlayer.stop(); played=[]; sources=[source("a",false),source("b",true)]; backend.independentEditing=false;
        browser.visible=true; browser.assemblyId=""; browser.editorMode=false; browser.sourceTab=2; browser.query=""; browser.category=""; browser.eventFilter=""; browser.auditionId=""; browser.auditionAsset=null; browser.refresh();
        waitForRendering(browser);
    }
    function test_selection_cannot_retarget_queued_audition() {
        browser.audition(sources[0]); browser.selectedAsset=sources[1];
        tryCompare(test,"played",["/test/a.wav"]); compare(browser.auditionAsset.id,"a");
        browser.audition(sources[0]); compare(browser.selectedAsset.id,"a"); verify(!materialPlayer.playing);
        browser.visible=false; verify(!materialPlayer.active);
    }
    function test_reveal_clears_filters_and_selects_project_material() {
        browser.editorMode=true; browser.assemblyId="project"; browser.sourceTab=1;
        browser.query="missing"; browser.category="music"; browser.eventFilter="rain";
        browser.revealAsset("b"); compare(browser.sourceTab,0); compare(browser.filteredAssets.length,2); compare(browser.selectedAsset.id,"b");
        compare(browser.query,""); compare(browser.category,""); compare(browser.eventFilter,"");
    }
    function test_no_result_recovery_and_visible_generated_identity() {
        browser.query="missing"; waitForRendering(browser);
        const clear=findChild(browser,"clearSourceFilters"); verify(clear.visible); mouseClick(clear);
        compare(browser.query,""); compare(browser.filteredAssets.length,2);
        const list=findChild(browser,"sourceList"); tryVerify(()=>list.itemAtIndex(1)!==null);
        const row=list.itemAtIndex(1); verify(row.height>=94);
        // Read the real source badge rather than inferring provenance from a path or category.
        const badge=findChild(row,"materialSourceDisclosure"); verify(badge.visible); verify(badge.generated);
    }
}
