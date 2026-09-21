//! Opt-in native scan, audition, atomic acceptance and portable-project contract.
import QtQuick
Item {
    id:smoke
    required property var shell
    required property var editor
    required property string fixtureRoot
    required property bool reopening
    property int stage:0
    property int ticks:0
    property int since:0
    property var facts:({})
    property string reportJson:""
    property string before:""
    function require(value,message){if(!value)throw new Error(message);}
    function exportResult(){
        facts.masks=editor.adjustment.effectMasks;facts.chain=editor.adjustment.effectChain;facts.reopening=reopening;
        require(facts.masks.length===4,"local repairs were not retained");
        const jobs=backend.jobStats();require(jobs.pending===0 && jobs.running===0 && jobs.done===0 && jobs.failed===0,"editor started global analysis jobs");facts.jobs=jobs;
        editor.auditionOriginal=false;editor.debugExport('file://'+fixtureRoot+(reopening?'/reopened.wav':'/repaired.wav'));stage=7;
    }
    function step(){
        const panel=editor.clickRepairPanel;
        switch(stage){
        case 0:
            if(independentEditor.busy || !shell.assets.length)return;
            require(backend.independentEditing,"not private editing");shell.chooseSource(0);editor.clickRepairFocus=true;stage=1;break;
        case 1:
            if(!editor.hasAsset)return;
            if(reopening){exportResult();return;}
            editor.adjustment.setDeClickParameter("sensitivity",85);
            require(shell.flushDrafts(),"baseline draft flush failed");
            editor.debugExport('file://'+fixtureRoot+'/baseline.wav');stage=2;break;
        case 2:
            if(renderExporter.running)return;
            require(renderExporter.hasResult,renderExporter.errorText || "baseline export failed");
            editor.timeline.hasTimeSelection=false;panel.scan();stage=3;since=ticks;break;
        case 3:
            if(clickAnalysis.running)return;
            require(!clickAnalysis.errorText,clickAnalysis.errorText);require(panel.candidates.length===4,"expected four injected click findings");
            facts.findings=panel.candidates;facts.scanSeconds=(ticks-since)*0.2;
            panel.selectAll();panel.locate(panel.candidates[0],true,true);since=ticks;stage=4;break;
        case 4:
            if(ticks-since<4)return;
            require(player.active && !player.errorText,"source audition failed");require(panel.selectedEntries.length===4,"audition invalidated selected findings");player.stop();
            before=JSON.stringify(editor.adjustment.snapshot());panel.applySelected();require(editor.adjustment.effectMasks.length===4,"batch repair failed");
            editor.undo();require(JSON.stringify(editor.adjustment.snapshot())===before,"batch repair did not undo in one step");editor.redo();
            panel.locate(panel.candidates[0],true,false);since=ticks;stage=5;break;
        case 5:
            if(ticks-since<4)return;
            require(player.active && !player.errorText && !editor.auditionOriginal,"adjusted audition failed");player.stop();
            require(shell.flushDrafts(),"draft flush failed");independentEditor.saveProject('file://'+fixtureRoot+'/click-repair.echo');stage=6;break;
        case 6:
            if(independentEditor.busy)return;
            require(!independentEditor.errorText,independentEditor.errorText);exportResult();break;
        case 7:
            if(renderExporter.running)return;
            require(renderExporter.hasResult,renderExporter.errorText || "repair export failed");facts.exportedPath=renderExporter.outputPath;
            reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer { interval:200; repeat:true; running:smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered:{
        try{if(++smoke.ticks>400)throw new Error("click review timed out");smoke.step();}
        catch(error){smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts});}
    } }
}
