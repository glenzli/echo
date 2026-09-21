//! Opt-in native Infer → scoped evidence → batch review → portable reopen contract.
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
    property int auditionTick: 0
    property real requestStarted: 0
    readonly property int sourceOffset: editor.asset && Number(editor.asset.durationMillis) > 1210000 ? 1200000 : 0
    function require(value,message) { if(!value)throw new Error(message); }
    function reviewText() {
        const panel=editor.transcriptPanel;
        panel.gapMode=false;panel.unitMode=true;
        require(panel.segments.length>2,"not enough timed units to review");
        const first=panel.segments[0], last=panel.segments[panel.segments.length-1];
        panel.searchText=first.text;
        require(panel.segments.some(s=>s.key===first.key),"literal text search lost its timed unit");
        panel.selectVisible();require(panel.selectedEntries.length>0,"search results cannot be checked");
        panel.searchText="";require(panel.selectedEntries.length===0,"filter kept invisible selections");
        panel.selectedKeys=[first.key,last.key];
        require(panel.selectedRanges.length===2,"separate words were merged across unselected speech");
        facts.batchRanges=panel.selectedRanges;
        beforeEdit=JSON.stringify(editor.adjustment.editSegments);
        panel.applySelection("keep");
        require(!panel.editError && editor.canUndo,"keep selected failed");editor.undo();
        require(JSON.stringify(editor.adjustment.editSegments)===beforeEdit,"keep selected did not undo atomically");
        panel.selectedKeys=[first.key,last.key];panel.applySelection("hide");
        require(!panel.editError && JSON.stringify(editor.adjustment.editSegments)!==beforeEdit,"batch hide failed");
        require(editor.canUndo,"batch edit cannot undo");editor.undo();
        require(JSON.stringify(editor.adjustment.editSegments)===beforeEdit && !editor.canUndo,"batch edit requires more than one undo");editor.redo();
        facts.editedSegments=editor.adjustment.editSegments;
        require(shell.flushDrafts(),"draft save failed");
        independentEditor.saveProject('file://'+fixtureRoot+'/transcript.echo');stage=3;
    }
    function step() {
        switch(stage) {
        case 0:
            if(independentEditor.busy || !shell.assets.length) return;
            require(backend.independentEditing,"not private editing");
            if(reopening) require(!shell.projectDirty, "AI project opened dirty");
            shell.chooseSource(0);editor.transcriptFocus=true;stage=1;break;
        case 1:
            if(!editor.hasAsset) return;
            const waveform=editor.waveformLevels;
            require(waveform.length>0,"source waveform not loaded");
            for(let refresh=0;refresh<8;++refresh) editor.refreshAsset();
            require(editor.waveformLevels===waveform,"metadata refresh replaced the immutable source waveform");
            facts.waveformReused=true;
            if(reopening) { stage=5;return; }
            const cleanBefore=editor.dirty;
            editor.timeline.exactTimeDialog.present(sourceOffset+1000,Math.min(sourceOffset+10000,Number(editor.asset.durationMillis)));
            editor.timeline.exactTimeDialog.apply();
            require(editor.timeline.hasTimeSelection && editor.timeline.selectionStartMillis===sourceOffset+1000, "exact selection did not reach the timeline");
            require(editor.dirty===cleanBefore,"navigation dirtied the audio draft");
            facts.exactTimeRange=true;facts.sourceOffset=sourceOffset;
            requestStarted=Date.now();
            editor.transcriptPanel.requestTranscription();stage=2;break;
        case 2:
            if(selectionTranscription.running) return;
            require(!selectionTranscription.errorText,selectionTranscription.errorText);
            require(!editor.transcriptPanel.notice,editor.transcriptPanel.notice);
            const records=backend.selectionTranscripts(editor.asset.id);
            require(records.length===1,"scoped result not accepted");
            require(records[0].start_millis===sourceOffset+1000,"source offset lost");
            require(records[0].transcript.segments.length>0,"no segment timing");
            require(records[0].alignment,"no real aligned timing available");
            facts.inferenceMillis=Date.now()-requestStarted;
            facts.evidence=records[0];facts.alignedUnitCount=records[0].alignment.items.length;
            editor.transcriptPanel.gapMode=true;
            facts.speechGaps=editor.transcriptPanel.segments;
            if(facts.speechGaps.length) {
                editor.transcriptPanel.toggleEntry(facts.speechGaps[0].key);
                editor.transcriptPanel.locate(facts.speechGaps[0],true);
                require(editor.auditionOriginal,"gap audition did not select original");
                require(editor.transcriptPanel.selectedEntries.length===1,"audition cleared the checked candidate");
                auditionTick=ticks;stage=4;
            } else reviewText();
            break;
        case 4:
            if(ticks-auditionTick<4) return;
            require(player.active && !player.errorText,"source-gap audition failed");
            facts.gapAuditionPosition=player.position;player.stop();
            beforeEdit=JSON.stringify(editor.adjustment.editSegments);
            editor.transcriptPanel.selectVisible();editor.transcriptPanel.applySelection("hide");
            require(!editor.transcriptPanel.editError && editor.canUndo,"gap batch cannot apply");
            editor.undo();require(JSON.stringify(editor.adjustment.editSegments)===beforeEdit && !editor.canUndo,"gap batch did not undo atomically");
            reviewText();break;
        case 3:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText,independentEditor.errorText);stage=5;break;
        case 5:
            const saved=backend.selectionTranscripts(editor.asset.id);
            require(saved.length===1,"saved transcript lost");
            require(editor.adjustment.editSegments.filter(s=>s.state===2).length===2,"batch text edit not restored");
            const jobs=backend.jobStats();
            require(jobs.pending===0 && jobs.running===0 && jobs.done===0 && jobs.failed===0,"global analysis jobs started");
            const record=editor.transcriptPanel.record;
            require(transcriptExporter.hasTiming(record), "real AI timing cannot be exported");
            for (const format of ["txt","srt","vtt"]) {
                const error=transcriptExporter.save(record, 'file://'+fixtureRoot+'/transcript.'+format, format, editor.asset.path);
                require(!error,error);
            }
            editor.transcriptPanel.gapMode=false; editor.transcriptPanel.unitMode=true;
            const entries=editor.transcriptPanel.segments;
            editor.transcriptPanel.selectedKeys=[entries[0].key,entries[entries.length-1].key];
            const selected=editor.transcriptPanel.selectedTranscript;
            require(selected && selected.segments.length===2,"selected transcript projection missing");
            require(selected.segments[0].start===entries[0].start,"selected export changed source time");
            for (const format of ["txt","srt","vtt"]) {
                const error=transcriptExporter.save(selected,'file://'+fixtureRoot+'/selected.'+format,format,editor.asset.path);
                require(!error,error);
            }
            facts.selectedTranscript=selected;
            editor.transcriptPanel.clearSelection();
            facts.transcriptExports=["txt","srt","vtt"];
            facts.evidence=saved[0];facts.editedSegments=editor.adjustment.editSegments;
            facts.scopedRecords=saved.length;facts.jobs=jobs;facts.reopening=reopening;
            editor.auditionOriginal=false;
            if(sourceOffset>0) { facts.longSourceDurationMillis=editor.asset.durationMillis; reportJson=JSON.stringify({ok:true,facts:facts}); return; }
            editor.debugExport('file://'+fixtureRoot+(reopening?'/reopened.wav':'/edited.wav'));stage=6;break;
        case 6:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,renderExporter.errorText || "edited export failed");
            facts.exportedPath=renderExporter.outputPath;
            reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer { interval:200; repeat:true; running:smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>400)throw new Error('editor AI timed out');smoke.step(); }
        catch(error) { smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
