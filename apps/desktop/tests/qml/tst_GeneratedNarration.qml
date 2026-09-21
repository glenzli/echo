import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test; name: "GeneratedNarration"; width: 960; height: 720; visible: true; when: windowShown
    property var accepted: []
    QtObject { id: backend; property bool independentEditing: false; function refresh() {} }
    QtObject { id: player; property bool playing: false; function togglePause() {} }
    QtObject { id: materialPlayer; function stop() {} }
    QtObject { id: inferencePrefs; property string runtimeEndpoint: "" }
    QtObject {
        id: generatedNarration
        property bool running: false; property bool accepting: false
        property string detailsJson: ""; property string errorText: ""; property url audioUrl: ""
        property var candidates: []; property string selectedCandidateId: ""; property var receipts: ({})
        signal accepted(string assetId)
        function discard() { detailsJson=""; errorText=""; candidates=[]; selectedCandidateId=""; receipts=({}); }
        function selectCandidate(id) { detailsJson=JSON.stringify(receipts[id]); selectedCandidateId=id; }
        function removeSelected() { candidates=candidates.filter(c=>c.id!==selectedCandidateId); if(candidates.length) selectCandidate(candidates[0].id); else { detailsJson=""; selectedCandidateId=""; } }
        function request(text,endpoint) { running=true; }
        function accept(assembly,global) { test.accepted.push([assembly,global]); accepted("new-source"); }
    }
    GeneratedNarrationDialog { id: dialog }
    function init() { dialog.close(); tryCompare(dialog,"visible",false); generatedNarration.running=false; generatedNarration.accepting=false; generatedNarration.discard();backend.independentEditing=false;test.accepted=[]; }
    function candidate() {
        generatedNarration.running=false;
        const id="candidate-"+generatedNarration.candidates.length;
        generatedNarration.receipts[id]={duration_millis:1100,input_text:findChild(dialog,"narrationText").text,runtime:{job:{physical_model:"local-tts"}}};
        generatedNarration.candidates=generatedNarration.candidates.concat([{id:id,text:generatedNarration.receipts[id].input_text}]);
        generatedNarration.selectCandidate(id);
    }
    function openDialog() { dialog.assemblyId=""; dialog.present();tryCompare(dialog,"opened",true); }
    function test_preview_required_and_accept_has_captured_target() {
        openDialog();findChild(dialog,"narrationText").text="An added memory.";
        mouseClick(findChild(dialog,"narrationGenerate"));verify(generatedNarration.running);candidate();
        verify(!findChild(dialog,"narrationAccept").enabled);dialog.reviewed=true;
        dialog.assemblyId="different-project";
        mouseClick(findChild(dialog,"narrationAccept"));compare(test.accepted.length,1);compare(test.accepted[0],["",true]);
    }
    function test_private_project_never_collects_into_library() {
        backend.independentEditing=true;openDialog();candidate();dialog.reviewed=true;
        mouseClick(findChild(dialog,"narrationAccept"));compare(test.accepted[0],["",false]);
    }
    function test_close_discards_candidate_and_text_is_locked_during_request() {
        openDialog();findChild(dialog,"narrationText").text="Test";
        mouseClick(findChild(dialog,"narrationGenerate"));verify(findChild(dialog,"narrationText").readOnly);
        candidate();verify(!findChild(dialog,"narrationText").readOnly);dialog.close();tryCompare(dialog,"visible",false);compare(generatedNarration.detailsJson,"");compare(test.accepted.length,0);
    }
    function test_each_candidate_needs_its_own_audition_and_capacity_is_visible() {
        openDialog(); candidate();
        const first=generatedNarration.selectedCandidateId;
        dialog.heardCandidates[first]=true; dialog.reviewed=true;
        candidate(); verify(!dialog.reviewed); verify(!findChild(dialog,"narrationAccept").enabled);
        generatedNarration.selectCandidate(first); verify(dialog.reviewed);
        candidate(); verify(!findChild(dialog,"narrationGenerate").enabled);
        mouseClick(findChild(dialog,"narrationRemove")); compare(generatedNarration.candidates.length,2);
        verify(findChild(dialog,"narrationGenerate").enabled);
    }
    function test_model_label_does_not_expose_cache_path() {
        openDialog(); generatedNarration.detailsJson=JSON.stringify({runtime:{job:{physical_model:"/private/cache/models--org--Qwen3-TTS/snapshots/fixed-build",model_profile:"tts"}}});
        compare(dialog.modelName(),"Qwen3-TTS");
    }
    function test_project_material_is_private_by_default_and_target_is_captured() {
        dialog.assemblyId="project-a"; dialog.present(); tryCompare(dialog,"opened",true);
        verify(!findChild(dialog,"narrationCollectGlobally").checked);
        candidate(); dialog.reviewed=true; dialog.assemblyId="project-b";
        mouseClick(findChild(dialog,"narrationAccept")); compare(test.accepted[0],["project-a",false]);
    }
    function test_character_limit_matches_unicode_backend() {
        openDialog(); findChild(dialog,"narrationText").text="  "+"😀".repeat(500)+"  ";
        compare(dialog.characterCount,500); verify(findChild(dialog,"narrationGenerate").enabled);
        findChild(dialog,"narrationText").text="😀".repeat(501);
        compare(dialog.characterCount,501); verify(!findChild(dialog,"narrationGenerate").enabled);
    }
    function test_project_candidate_actions_fit_in_dialog() {
        dialog.assemblyId="project"; dialog.present(); tryCompare(dialog,"opened",true);
        candidate(); generatedNarration.errorText="A recoverable problem. Please try again.";
        waitForRendering(dialog.contentItem);
        const accept=findChild(dialog,"narrationAccept");
        verify(accept.mapToItem(dialog.contentItem,0,accept.height).y<=dialog.contentItem.height+1);
        verify(findChild(dialog,"narrationText").height>=100);
    }

}
