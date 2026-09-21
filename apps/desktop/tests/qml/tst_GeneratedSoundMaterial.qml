import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test; name: "GeneratedSoundMaterial"; width: 960; height: 720; visible: true; when: windowShown
    property var accepted: []
    property var requested: []
    QtObject { id: backend; property bool independentEditing: false; function refresh() {} }
    QtObject { id: player; property bool playing: false; function togglePause() {} }
    QtObject { id: materialPlayer; function stop() {} }
    QtObject { id: inferencePrefs; property string runtimeEndpoint: "" }
    QtObject {
        id: generatedSoundMaterial
        property bool running: false; property bool accepting: false
        property string detailsJson: ""; property string errorText: ""; property url audioUrl: ""
        property var candidates: []; property string selectedCandidateId: ""; property var receipts: ({})
        signal accepted(string assetId)
        function discard() { detailsJson=""; errorText=""; candidates=[]; selectedCandidateId=""; receipts=({}); }
        function selectCandidate(id) { detailsJson=JSON.stringify(receipts[id]); selectedCandidateId=id; }
        function removeSelected() { candidates=candidates.filter(c=>c.id!==selectedCandidateId); if(candidates.length) selectCandidate(candidates[0].id); else { detailsJson=""; selectedCandidateId=""; } }
        function requestSoundMaterial(text,seconds,ambient,endpoint,model) { test.requested=[text,seconds,ambient,endpoint,model]; running=true; }
        function accept(assembly,global) { test.accepted.push([assembly,global]); accepted("new-source"); }
    }
    GeneratedSoundMaterialDialog { id: dialog }
    function init() { dialog.close(); tryCompare(dialog,"visible",false); generatedSoundMaterial.running=false; generatedSoundMaterial.accepting=false; generatedSoundMaterial.discard();backend.independentEditing=false;test.accepted=[]; }
    function candidate() {
        generatedSoundMaterial.running=false;
        const id="candidate-"+generatedSoundMaterial.candidates.length;
        generatedSoundMaterial.receipts[id]={generation_kind:dialog.musicMode ? "music" : "sound_effect",material_category:dialog.musicMode ? "music" : findChild(dialog,"soundMaterialAmbience").checked ? "ambience" : "effects",request:{model_choice:dialog.modelChoice,duration_seconds:findChild(dialog,"soundMaterialDuration").value,seed:42},duration_millis:10000,input_text:findChild(dialog,"soundMaterialPrompt").text,runtime:{job:{physical_model:"local-tts"}}};
        generatedSoundMaterial.candidates=generatedSoundMaterial.candidates.concat([{id:id,text:generatedSoundMaterial.receipts[id].input_text}]);
        generatedSoundMaterial.selectCandidate(id);
    }
    function openDialog() { dialog.assemblyId=""; dialog.present();tryCompare(dialog,"opened",true); }
    function test_preview_required_and_accept_has_captured_target() {
        openDialog();findChild(dialog,"soundMaterialPrompt").text="An added memory.";
        mouseClick(findChild(dialog,"soundMaterialGenerate"));verify(generatedSoundMaterial.running);candidate();
        verify(!findChild(dialog,"soundMaterialAccept").enabled);dialog.reviewed=true;
        dialog.assemblyId="different-project";
        mouseClick(findChild(dialog,"soundMaterialAccept"));compare(test.accepted.length,1);compare(test.accepted[0],["",true]);
    }
    function test_private_project_never_collects_into_library() {
        backend.independentEditing=true;openDialog();candidate();dialog.reviewed=true;
        mouseClick(findChild(dialog,"soundMaterialAccept"));compare(test.accepted[0],["",false]);
    }
    function test_close_discards_candidate_and_text_is_locked_during_request() {
        openDialog();findChild(dialog,"soundMaterialPrompt").text="Test";
        mouseClick(findChild(dialog,"soundMaterialGenerate"));verify(findChild(dialog,"soundMaterialPrompt").readOnly);
        candidate();verify(!findChild(dialog,"soundMaterialPrompt").readOnly);dialog.close();tryCompare(dialog,"visible",false);compare(generatedSoundMaterial.detailsJson,"");compare(test.accepted.length,0);
    }
    function test_each_candidate_needs_its_own_audition_and_capacity_is_visible() {
        openDialog(); candidate();
        const first=generatedSoundMaterial.selectedCandidateId;
        dialog.heardCandidates[first]=true; dialog.reviewed=true;
        candidate(); verify(!dialog.reviewed); verify(!findChild(dialog,"soundMaterialAccept").enabled);
        generatedSoundMaterial.selectCandidate(first); verify(dialog.reviewed);
        candidate(); verify(!findChild(dialog,"soundMaterialGenerate").enabled);
        mouseClick(findChild(dialog,"soundMaterialRemove")); compare(generatedSoundMaterial.candidates.length,2);
        verify(findChild(dialog,"soundMaterialGenerate").enabled);
    }
    function test_model_label_does_not_expose_cache_path() {
        openDialog(); generatedSoundMaterial.detailsJson=JSON.stringify({request:{seed:42},runtime:{job:{physical_model:"/private/cache/models--org--Qwen3-TTS/snapshots/fixed-build",model_profile:"tts"}}});
        compare(dialog.modelName(),"Qwen3-TTS");
        generatedSoundMaterial.detailsJson=JSON.stringify({request:{seed:42},runtime:{job:{model_profile:"stable_audio_3_sm_sfx",physical_model:"stabilityai/stable-audio-3-optimized@pinned:sm-sfx"}}});
        compare(dialog.modelName(),"Stable Audio 3 · Small-SFX");
    }
    function test_project_material_is_private_by_default_and_target_is_captured() {
        dialog.assemblyId="project-a"; dialog.present(); tryCompare(dialog,"opened",true);
        verify(!findChild(dialog,"soundMaterialCollectGlobally").checked);
        candidate(); dialog.reviewed=true; dialog.assemblyId="project-b";
        mouseClick(findChild(dialog,"soundMaterialAccept")); compare(test.accepted[0],["project-a",false]);
    }
    function test_character_limit_matches_unicode_backend() {
        openDialog(); findChild(dialog,"soundMaterialPrompt").text="  "+"😀".repeat(500)+"  ";
        compare(dialog.characterCount,500); verify(findChild(dialog,"soundMaterialGenerate").enabled);
        findChild(dialog,"soundMaterialPrompt").text="😀".repeat(501);
        compare(dialog.characterCount,501); verify(!findChild(dialog,"soundMaterialGenerate").enabled);
    }
    function test_empty_prompt_is_compact_and_review_expands_without_clipping() {
        openDialog(); compare(dialog.height,520);
        const height=dialog.height; candidate(); waitForRendering(dialog.contentItem);
        verify(dialog.height>height); verify(findChild(dialog,"soundMaterialPrompt").height>=100);
        const accept=findChild(dialog,"soundMaterialAccept");
        verify(accept.mapToItem(dialog.contentItem,accept.width,accept.height).x<=dialog.contentItem.width+1);
        verify(accept.mapToItem(dialog.contentItem,0,accept.height).y<=dialog.contentItem.height+1);
    }
    function test_preset_and_duration_route_to_sound_generation() {
        openDialog(); const preset=findChild(dialog,"soundMaterialPreset");
        preset.currentIndex=1; preset.activated(1);
        verify(findChild(dialog,"soundMaterialPrompt").text.indexOf("rain")>=0);
        verify(findChild(dialog,"soundMaterialAmbience").checked);
        findChild(dialog,"soundMaterialDuration").value=7;
        mouseClick(findChild(dialog,"soundMaterialGenerate"));
        compare(test.requested[1],7); compare(test.requested[2],true);
        verify(!findChild(dialog,"soundMaterialDuration").enabled);
    }
    function test_switching_candidates_restores_settings_without_reusing_review() {
        openDialog(); findChild(dialog,"soundMaterialPrompt").text="rain";
        findChild(dialog,"soundMaterialDuration").value=7;
        findChild(dialog,"soundMaterialAmbience").checked=true; candidate();
        const first=generatedSoundMaterial.selectedCandidateId;
        tryCompare(findChild(dialog,"soundMaterialDuration"),"value",7);
        findChild(dialog,"soundMaterialPrompt").text="door";
        findChild(dialog,"soundMaterialDuration").value=2;
        findChild(dialog,"soundMaterialAmbience").checked=false; candidate();
        wait(1); generatedSoundMaterial.selectCandidate(first);
        tryCompare(findChild(dialog,"soundMaterialPrompt"),"text","rain");
        compare(findChild(dialog,"soundMaterialDuration").value,7);
        verify(findChild(dialog,"soundMaterialAmbience").checked);
        verify(!findChild(dialog,"soundMaterialAccept").enabled);
    }
    function test_project_candidate_actions_fit_in_dialog() {
        dialog.assemblyId="project"; dialog.present(); tryCompare(dialog,"opened",true);
        candidate(); generatedSoundMaterial.errorText="A recoverable problem. Please try again.";
        waitForRendering(dialog.contentItem);
        const accept=findChild(dialog,"soundMaterialAccept");
        verify(accept.mapToItem(dialog.contentItem,0,accept.height).y<=dialog.contentItem.height+1);
        verify(findChild(dialog,"soundMaterialPrompt").height>=100);
    }

    function test_music_model_and_presets_route_and_restore_candidate_identity() {
        dialog.present("stable_audio_3_small_music"); tryCompare(dialog,"opened",true);
        verify(dialog.musicMode); verify(!findChild(dialog,"soundMaterialAmbience").visible);
        const preset=findChild(dialog,"soundMaterialPreset"); preset.currentIndex=1; preset.activated(1);
        verify(findChild(dialog,"soundMaterialPrompt").text.indexOf("piano")>=0);
        mouseClick(findChild(dialog,"soundMaterialGenerate"));
        compare(test.requested[4],"stable_audio_3_small_music"); compare(test.requested[2],false);
        verify(!findChild(dialog,"soundMaterialModel").enabled); candidate();
        const music=generatedSoundMaterial.selectedCandidateId; wait(1);
        findChild(dialog,"soundMaterialModel").currentIndex=0;
        findChild(dialog,"soundMaterialPrompt").text="rain"; candidate(); wait(1);
        generatedSoundMaterial.selectCandidate(music);
        tryCompare(dialog,"musicMode",true); verify(!dialog.reviewed);
        verify(findChild(dialog,"soundMaterialPrompt").text.indexOf("piano")>=0);
    }

}
