import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test; name: "SourceDisclosure"; width: 960; height: 700; visible: true; when: windowShown
    property var writes: []
    property string failure: ""
    property var asset: ({id:"source",durationMillis:10000,sourceDisclosure:{sources:[]}})
    QtObject { id: catalog
        function setSourceDisclosure(id,revision,spans) { test.writes.push({id:id,revision:revision,spans:JSON.parse(JSON.stringify(spans))}); return test.failure; }
    }
    SourceDisclosureDialog { id: dialog; catalogBackend: catalog }
    SourceDisclosureBadge { id: badge; asset: test.asset; editable: true; onActivated: dialog.present(test.asset,1200,2400) }
    function init() { dialog.close();tryCompare(dialog,"visible",false); failure=""; writes=[]; asset={id:"source",durationMillis:10000,sourceDisclosure:{sources:[]}}; }
    function openDialog() { mouseClick(badge); tryCompare(dialog,"opened",true); waitForRendering(dialog.contentItem); }
    function test_selection_whole_source_remove_and_commit() {
        openDialog(); dialog.noteText="Recreated rain";
        mouseClick(findChild(dialog,"disclosureAddSelection")); compare(dialog.spans.length,1);compare(dialog.spans[0].startMillis,1200);compare(dialog.spans[0].endMillis,2400);
        dialog.kindIndex=2;mouseClick(findChild(dialog,"disclosureAddWhole"));compare(dialog.spans[1].kind,"reconstructed_speech");compare(dialog.spans[1].endMillis,10000);
        tryVerify(()=>dialog.labelList.itemAtIndex(0)!==null);waitForRendering(dialog.contentItem);mouseClick(findChild(dialog.labelList.itemAtIndex(0),"disclosureRemove0"));compare(dialog.spans.length,1);
        mouseClick(findChild(dialog,"disclosureSave"));tryCompare(dialog,"opened",false);compare(writes.length,1);compare(writes[0].revision,0);compare(writes[0].spans.length,1);
    }
    function test_cancel_and_stale_save_keep_persisted_labels() {
        asset={id:"source",durationMillis:10000,sourceDisclosure:{sources:[{assetId:"source",revisionId:29,spans:[{kind:"ai_processed",startMillis:0,endMillis:500,note:"denoise"}]}]}};
        openDialog();dialog.append(true);compare(asset.sourceDisclosure.sources[0].spans.length,1);dialog.close();compare(writes.length,0);
        openDialog();compare(dialog.spans.length,1);failure="Changed elsewhere";mouseClick(findChild(dialog,"disclosureSave"));compare(writes[0].revision,29);verify(dialog.opened);compare(dialog.errorText,failure);compare(dialog.spans.length,1);
    }
    function test_imported_labels_are_explained_and_clearable() {
        asset={id:"source",durationMillis:10000,sourceDisclosure:{sources:[{assetId:"source",revisionId:0,origin:"embedded_export",spans:[{kind:"ai_generated",startMillis:0,endMillis:10000,note:""}]}]}};
        openDialog();verify(dialog.importedLabels);compare(dialog.expectedRevision,0);
        tryVerify(()=>dialog.labelList.itemAtIndex(0)!==null);waitForRendering(dialog.contentItem);
        mouseClick(findChild(dialog.labelList.itemAtIndex(0),"disclosureRemove0"));compare(dialog.spans.length,0);
        mouseClick(findChild(dialog,"disclosureSave"));compare(writes[0].revision,0);compare(writes[0].spans.length,0);
    }
    function test_label_budget_and_readonly_mix() {
        openDialog();for(let i=0;i<65;++i)dialog.append(false);compare(dialog.spans.length,64);verify(!findChild(dialog,"disclosureAddWhole").enabled);dialog.close();
        asset={id:"mix",assemblyId:"mix",durationMillis:10000,sourceDisclosure:{sources:[{assetId:"source",revisionId:1,spans:[{kind:"ai_generated",startMillis:0,endMillis:500,note:"rain"}]}]}};
        openDialog();verify(dialog.readOnly);compare(dialog.spans.length,1);dialog.save();compare(writes.length,0);
    }
    function test_runtime_generation_receipt_is_mandatory() {
        asset={id:"source",durationMillis:10000,sourceDisclosure:{sources:[{assetId:"source",revisionId:0,origin:"runtime_generated",generation:{input_text:"Later narration",runtime:{job:{physical_model:"tts",model_build:"build"}}},spans:[{kind:"ai_generated",startMillis:0,endMillis:10000,note:""}]}]}};
        openDialog(); verify(dialog.readOnly); compare(dialog.generation.input_text,"Later narration");
        verify(!findChild(dialog,"disclosureSave").visible); dialog.save(); compare(writes.length,0);
    }
    function test_sound_receipt_displays_prompt_duration_seed_and_cannot_be_cleared_data() {
        return [{tag:"sound",kind:"sound_effect"},{tag:"music",kind:"music"}];
    }
    function test_sound_receipt_displays_prompt_duration_seed_and_cannot_be_cleared(data) {
        asset={id:"source",durationMillis:8000,sourceDisclosure:{sources:[{assetId:"source",revisionId:0,origin:"runtime_generated",generation:{generation_kind:data.kind,input_text:"Quiet rain",request:{duration_seconds:8,seed:4294967295},runtime:{job:{physical_model:"small-sfx",model_build:"fixed"}}},spans:[{kind:"ai_generated",startMillis:0,endMillis:8000,note:""}]}]}};
        openDialog(); mouseClick(findChild(dialog,"generationRecordToggle"));
        const record=findChild(dialog,"generationRecordText").text;
        verify(record.indexOf("Quiet rain")>=0); verify(record.indexOf("4294967295")>=0);
        verify(record.indexOf("8 seconds")>=0); verify(dialog.readOnly);
        dialog.save(); compare(writes.length,0);
    }
    function test_full_generation_record_can_be_read_without_clipping() {
        const narration="这是完整的旁白。".repeat(60)+"最后一句";
        asset={id:"source",durationMillis:10000,sourceDisclosure:{sources:[{assetId:"source",revisionId:0,origin:"runtime_generated",generation:{input_text:narration,runtime:{job:{physical_model:"/cache/"+"model/".repeat(30),model_build:"build"}}},spans:[{kind:"ai_generated",startMillis:0,endMillis:10000,note:""}]}]}};
        openDialog(); verify(!findChild(dialog,"generationRecordScroll").visible);
        mouseClick(findChild(dialog,"generationRecordToggle")); waitForRendering(dialog.contentItem);
        const record=findChild(dialog,"generationRecordText"), scroll=findChild(dialog,"generationRecordScroll");
        verify(scroll.visible); verify(record.readOnly); verify(record.selectByMouse);
        verify(record.text.endsWith(narration)); verify(record.height>scroll.height);
        dialog.close(); openDialog(); verify(!dialog.recordExpanded);
    }

    function test_prepared_sound_record_preserves_original_and_both_jobs() {
        asset={id:"source",durationMillis:3000,sourceDisclosure:{sources:[{assetId:"source",revisionId:0,origin:"runtime_generated",generation:{generation_kind:"sound_effect",input_text:"雨声，不要音乐",request:{duration_seconds:3,seed:7},prompt_preparation:{effective_prompt:"Rain, no music",rules_revision:"rules-1",text_job:{physical_model:"local-text",id:"text-job-1"}},runtime:{job:{physical_model:"small-sfx",model_build:"fixed",id:"sound-job-2"}}},spans:[{kind:"ai_generated",startMillis:0,endMillis:3000,note:""}]}]}};
        openDialog(); mouseClick(findChild(dialog,"generationRecordToggle"));
        const record=findChild(dialog,"generationRecordText").text;
        for (const value of ["雨声，不要音乐","Rain, no music","rules-1","local-text","text-job-1","sound-job-2"])
            verify(record.indexOf(value)>=0,value);
        verify(dialog.readOnly); dialog.save(); compare(writes.length,0);
    }

}
