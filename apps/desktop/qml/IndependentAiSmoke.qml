//! Opt-in native Infer → scoped evidence → edit → portable reopen contract.
import QtQuick
Item {
    id: smoke
    required property var shell
    required property var editor
    required property string fixtureRoot
    required property bool reopening
    property int stage: 0
    property int ticks: 0
    property string reportJson: ""
    property var facts: ({})
    property string beforeEdit: ""
    function require(value,message) { if(!value)throw new Error(message); }
    function step() {
        switch(stage) {
        case 0:
            if(independentEditor.busy || !shell.assets.length) return;
            require(backend.independentEditing,"not private editing");
            shell.chooseSource(0); editor.transcriptFocus=true;
            stage=1;break;
        case 1:
            if(!editor.hasAsset) return;
            if(reopening) { stage=5;return; }
            editor.timeline.selectionStartMillis=1000;
            editor.timeline.selectionEndMillis=Math.min(10000,Number(editor.asset.durationMillis));
            editor.timeline.hasTimeSelection=true;
            editor.transcriptPanel.requestTranscription();stage=2;break;
        case 2:
            if(selectionTranscription.running) return;
            require(!selectionTranscription.errorText,selectionTranscription.errorText);
            require(!editor.transcriptPanel.notice,editor.transcriptPanel.notice);
            const records=backend.selectionTranscripts(editor.asset.id);
            require(records.length===1,"scoped result not accepted");
            require(records[0].start_millis===1000,"source offset lost");
            require(records[0].transcript.segments.length>0,"no segment timing");
            facts.evidence=records[0];
            if(records[0].alignment) {
                editor.transcriptPanel.unitMode=true;
                facts.alignedUnitCount=records[0].alignment.items.length;
                require(editor.transcriptPanel.segments.length>0,"aligned units missing from editor");
            }
            const segment=editor.transcriptPanel.segments[0];
            require(segment!==undefined,"transcript missing from editor");
            const range=editor.transcriptPanel.bounds(segment);
            editor.transcriptPanel.rangeRequested(range.start,range.end,false);
            require(editor.timeline.hasTimeSelection && editor.timeline.selectionStartMillis===range.start,"text selection did not reach timeline");
            beforeEdit=JSON.stringify(editor.adjustment.editSegments);
            editor.transcriptPanel.editRequested("hide",range.start,range.end);
            require(JSON.stringify(editor.adjustment.editSegments)!==beforeEdit,"text edit did not hide source span");
            require(editor.canUndo,"text edit cannot undo");editor.undo();
            require(JSON.stringify(editor.adjustment.editSegments)===beforeEdit,"text edit undo changed source coordinates");editor.redo();
            require(shell.flushDrafts(),"draft save failed");
            independentEditor.saveProject('file://'+fixtureRoot+'/transcript.echo');stage=3;break;
        case 3:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText,independentEditor.errorText);
            stage=5;break;
        case 5:
            const saved=backend.selectionTranscripts(editor.asset.id);
            require(saved.length===1,"saved transcript lost");
            require(editor.adjustment.editSegments.some(s=>s.state===2),"text edit not restored");
            const jobs=backend.jobStats();
            require(jobs.pending===0 && jobs.running===0 && jobs.done===0 && jobs.failed===0,"global analysis jobs started");
            facts.evidence=saved[0];
            facts.scopedRecords=saved.length;facts.jobs=jobs;facts.reopening=reopening;
            reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer { interval:200; repeat:true; running:smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>400)throw new Error('editor AI timed out');smoke.step(); }
        catch(error) { smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
