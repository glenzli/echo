import QtQuick
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "TranscriptReview"
    width: 1080; height: 460
    visible: true
    when: windowShown
    property var asset: ({id:"source",durationMillis:10000})
    property var located: []
    property var lastResult: null
    QtObject {
        id: backend
        signal assetsChanged()
        function selectionTranscripts(id) { return []; }
        function transcriptsForAsset(id) { return []; }
    }
    QtObject {
        id: selectionTranscription
        property bool running: false
        property bool discarded: false
        property string requestKey: ""
        property string resultJson: ""
        property string errorText: ""
        signal resultChanged()
        function discard() { requestKey=""; }
    }
    QtObject {
        id: transcriptExporter
        property string copied: ""
        property var saved: null
        function hasTiming(record) { return record.segments.length > 0; }
        function copyText(record) { copied=record.text; return ""; }
        function save(record, url, format, source) { saved={record:record,url:url,format:format,source:source}; return ""; }
    }
    QtObject { id: inferencePrefs; property string runtimeEndpoint: "" }
    SoundAdjustmentDraft { id: draft; asset: test.asset }
    SoundTranscriptPanel {
        id: panel
        anchors.fill: parent
        asset: test.asset; revisionKey: "draft"; rangeStart: 0; rangeEnd: 10000
        trimStart: draft.trimStartMillis; trimEnd: draft.trimEndMillis
        onRangeRequested: (start,end,play) => test.located.push([start,end,play])
        onEditsRequested: (kind,ranges) => {
            test.lastResult=draft.editSourceRanges(ranges,kind);
            panel.finishEdit(test.lastResult);
        }
    }
    function init() {
        draft.resetFromAsset(); panel.gapMode=false; panel.unitMode=false;
        panel.searchText=""; panel.boundaryMargin=80; panel.minimumGap=600;
        panel.recordIndex=0; panel.revisionKey="draft";
        panel.records=[{label:"Recording",model:"test",text:"Soft rain. Slow steps.",segments:[
            {text:"Soft rain.",start:1,end:2},{text:"Slow steps.",start:4,end:5},{text:"Soft rain again.",start:6,end:7}],
            units:[{text:"Soft",start:1,end:1.4},{text:"rain",start:1.4,end:2},{text:"Slow",start:4,end:4.4},{text:"steps",start:4.4,end:5},{text:"Soft",start:6,end:6.4},{text:"rain",start:6.4,end:7}]}];
        panel.clearSelection(); located=[];lastResult=null;wait(30);
    }
    function test_copy_selected_preserves_original_timing_without_boundary_margin() {
        panel.toggleEntry("text:1");
        mouseClick(findChild(panel,"copyTranscript"));compare(transcriptExporter.copied,"Slow steps.");
        compare(panel.selectedTranscript.segments,[{text:"Slow steps.",start:4,end:5}]);
        panel.gapMode=true;compare(panel.selectedTranscript,null);
    }
    function test_transcription_limit_matches_available_button() {
        panel.rangeStart=0;panel.rangeEnd=300001;
        verify(!findChild(panel,"transcribeSelectionButton").enabled);
        panel.rangeEnd=10000;verify(findChild(panel,"transcribeSelectionButton").enabled);
    }
    function test_text_only_result_remains_readable_and_copyable() {
        panel.records=[{label:"Recording",model:"text-model",text:"只有文字的识别结果。",segments:[],units:[]}];
        verify(panel.textOnly);
        const text=findChild(panel,"untimedTranscript");
        verify(text.visible); compare(text.text,"只有文字的识别结果。");
        mouseClick(findChild(panel,"copyTranscript"));
        compare(transcriptExporter.copied,text.text);
        verify(!findChild(panel,"hideTranscriptSelection").enabled);
    }
    function test_copy_includes_complete_record_despite_search() {
        panel.searchText="rain";
        mouseClick(findChild(panel,"copyTranscript"));
        compare(transcriptExporter.copied,panel.record.text);
    }
    function test_search_check_and_one_step_undo() {
        const input=findChild(panel,"transcriptSearch");mouseClick(input);keyClick(Qt.Key_R);keyClick(Qt.Key_A);keyClick(Qt.Key_I);keyClick(Qt.Key_N);
        compare(panel.segments.length,2);
        mouseClick(findChild(panel,"selectTranscriptResults"));compare(panel.selectedEntries.length,2);
        const before=JSON.stringify(draft.editSegments);
        mouseClick(findChild(panel,"hideTranscriptSelection"));verify(lastResult.ok);
        compare(draft.editSegments.filter(s=>s.state===2).length,2);
        verify(draft.canUndo);draft.undo();compare(JSON.stringify(draft.editSegments),before);verify(!draft.canUndo);
        draft.redo();compare(draft.editSegments.filter(s=>s.state===2).length,2);
    }
    function test_check_clear_and_granularity_invalidation() {
        const check=findChild(panel,"check-text:0");
        check.forceActiveFocus();keyClick(Qt.Key_Space);compare(panel.selectedEntries.length,1);
        compare(check.indicator.width,16);verify(check.indicator.x>=0);
        panel.searchText="slow";compare(panel.selectedEntries.length,0);
        panel.selectVisible();panel.unitMode=true;compare(panel.selectedEntries.length,0);
        panel.selectVisible();panel.boundaryMargin=120;compare(panel.selectedEntries.length,0);
        panel.selectVisible();panel.revisionKey="new";compare(panel.selectedEntries.length,0);
        panel.selectVisible();mouseClick(findChild(panel,"clearTranscriptSelection"));compare(panel.selectedEntries.length,0);
    }
    function test_phrase_units_and_keep_complement() {
        mouseClick(findChild(panel,"transcriptUnits"));verify(panel.unitMode);
        panel.searchText="soft rain";compare(panel.segments.length,4);
        panel.selectVisible();compare(panel.selectedRanges.length,2);
        mouseClick(findChild(panel,"keepTranscriptSelection"));verify(lastResult.ok);
        compare(draft.editSegments.filter(s=>s.state===0).length,2);
        draft.undo();compare(draft.editSegments.length,1);
    }
    function test_gap_review_uses_context_only_for_audition() {
        const mode=findChild(panel,"transcriptMode");mouseClick(mode,mode.width*0.75,mode.height/2);verify(panel.gapMode);
        compare(panel.segments.length,2);
        mouseClick(findChild(panel,"audition-gap:1"));compare(located[0],[1830,4170,true]);
        mouseClick(findChild(panel,"check-gap:1"));compare(panel.selectedRanges,[{start:2080,end:3920}]);
        mouseClick(findChild(panel,"hideTranscriptSelection"));verify(lastResult.ok);
        const hidden=draft.editSegments.filter(s=>s.state===2);compare(hidden.length,1);
        compare(hidden[0].sourceStartMillis,2080);compare(hidden[0].sourceEndMillis,3920);
        draft.undo();compare(draft.editSegments.length,1);
    }
    function test_rejected_empty_and_large_batches_leave_history_untouched() {
        panel.records=[{label:"All",model:"test",text:"all",segments:[{text:"all",start:0,end:10}],units:[]}];
        panel.selectVisible();mouseClick(findChild(panel,"hideTranscriptSelection"));compare(lastResult.error,"empty");
        verify(panel.editError.length>0);verify(!draft.canUndo);compare(panel.selectedEntries.length,1);
        panel.boundaryMargin=0;
        const units=[];for(let i=0;i<64;++i)units.push({text:"word",start:0.01+i*0.1,end:0.02+i*0.1});
        panel.records=[{label:"Many",model:"test",text:"many",segments:units,units:[]}];
        panel.selectVisible();mouseClick(findChild(panel,"hideTranscriptSelection"));compare(lastResult.error,"limit");
        verify(!draft.canUndo);compare(draft.editSegments.length,1);compare(panel.selectedEntries.length,64);
    }
    function test_repeat_hide_is_noop() {
        panel.toggleEntry("text:0");panel.applySelection("hide");verify(lastResult.changed);
        const history=draft._history.length;
        panel.toggleEntry("text:0");panel.applySelection("hide");verify(lastResult.ok);verify(!lastResult.changed);
        compare(draft._history.length,history);
    }
    function test_large_aligned_result_selection_and_search_are_bounded() {
        const units=[];
        for(let i=0;i<20000;++i)units.push({text:i%2 ? "rain" : "soft",start:i/2000,end:(i+1)/2000});
        const start=Date.now();
        panel.records=[{label:"Large",model:"test",text:"test",segments:[],units:units}];panel.unitMode=true;
        panel.searchText="soft rain";compare(panel.segments.length,20000);
        panel.selectVisible();compare(panel.selectedEntries.length,20000);compare(panel.selectedRanges.length,1);
        verify(Date.now()-start<2500,"Large alignment blocked the UI for more than 2.5 seconds");
        panel.records=[];compare(panel.selectedEntries.length,0);compare(panel.selectedKeys.length,0);
    }
}
