//! Opt-in packaged source disclosure, unchanged audio and portable-project contract.
import QtQuick
Item {
    id: smoke
    required property var shell
    required property var editor
    required property string fixtureRoot
    required property bool reopening
    property var library: null
    property var generatedSource: null
    property int stage: 0
    property int ticks: 0
    property int since: 0
    property string reportJson: ""
    property var facts: ({})
    property string before: ""
    function require(value,message) { if(!value) throw new Error(message); }
    function tapeStep() {
        switch(stage) {
        case 0: {
            const sources=backend.listAssets().filter(a=>!a.assemblyId);
            require(sources.length>=2,"Tape fixture needs two sources");
            generatedSource=sources.find(a=>a.path.indexOf("synthetic")>=0) || sources[0];
            const revisions=generatedSource.sourceDisclosure.sources;
            const revision=revisions.length ? revisions[0].revisionId : 0;
            require(!backend.setSourceDisclosure(generatedSource.id,revision,[{kind:"ai_generated",startMillis:0,endMillis:generatedSource.durationMillis,note:"Declared synthetic Tape fixture"}]),"Tape label save failed");
            require(!backend.setSoundMembership(generatedSource.id,true,true,"ambience"),"Tape membership failed");
            library.debugOpenSoundTape();library.refreshAssets();
            require(!library.includeGeneratedSources,"Tape did not default to exclusion");
            require(!library.filteredAssets.some(a=>a.id===generatedSource.id),"generated source leaked into default Tape");
            facts.defaultExcluded=library.excludedGeneratedCount;
            library.includeGeneratedSources=true;stage=1;break;
        }
        case 1:
            require(library.filteredAssets.some(a=>a.id===generatedSource.id),"include switch did not restore source");
            library.setSearchText(generatedSource.path.split("/").pop());
            require(library.filteredAssets.some(a=>a.id===generatedSource.id),"fixture search failed");
            library.debugPlaySoundTape();since=ticks;stage=2;break;
        case 2:
            if(ticks-since<5) return;
            require(player.active && !player.errorText,"Tape source did not play");
            library.includeGeneratedSources=false;since=ticks;stage=3;break;
        case 3:
            if(ticks-since<3) return;
            require(!player.active,"excluded source kept playing");
            library.setSearchText("");library.includeGeneratedSources=true;library.tapeScope="originals";
            require(!library.filteredAssets.some(a=>a.id===generatedSource.id),"Original Tape included a generated source");
            library.tapeScope="materials";library.includeGeneratedSources=false;
            require(!library.filteredAssets.some(a=>a.id===generatedSource.id),"Material Tape exclusion failed");
            library.includeGeneratedSources=true;
            require(library.filteredAssets.some(a=>a.id===generatedSource.id),"Material Tape inclusion failed");
            library.tapeScope="memories";facts.stoppedExcludedPlayback=true;stage=6;since=ticks;break;
        case 6:
            if(ticks-since<4) return;
            facts.includedCount=library.filteredAssets.length;
            reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    function step() {
        if(library) {tapeStep();return;}
        switch(stage) {
        case 0:
            if(independentEditor.busy || !shell.assets.length) return;
            require(backend.independentEditing,"not private editing"); shell.chooseSource(0); stage=1; break;
        case 1:
            if(!editor.hasAsset) return;
            before=JSON.stringify(editor.adjustment.snapshot());
            if(reopening) { require(editor.asset.hasGeneratedSource,"portable source label missing");stage=5;return; }
            editor.debugExport('file://'+fixtureRoot+'/baseline.wav');stage=2;break;
        case 2:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,renderExporter.errorText || "baseline export failed");
            editor.presentSourceDisclosure();editor.sourceDisclosureDialog.noteText="Declared synthetic validation source";editor.sourceDisclosureDialog.append(true);since=ticks;stage=3;break;
        case 3:
            if(ticks-since<5) return;
            editor.sourceDisclosureDialog.save();require(!editor.sourceDisclosureDialog.errorText,"label save failed");
            require(shell.projectDirty,"source labels did not dirty portable project");
            require(JSON.stringify(editor.adjustment.snapshot())===before,"labels altered audio draft");
            require(shell.flushDrafts(),"draft flush failed");independentEditor.saveProject('file://'+fixtureRoot+'/disclosed.echo');stage=4;break;
        case 4:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText,independentEditor.errorText);stage=5;break;
        case 5:
            require(editor.asset.hasGeneratedSource,"source projection is missing disclosure");
            facts.sourceDisclosure=editor.asset.sourceDisclosure;
            editor.debugExport('file://'+fixtureRoot+(reopening?'/reopened.wav':'/marked.wav'));stage=6;break;
        case 6:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,renderExporter.errorText || "export failed");
            facts.jobs=backend.jobStats();require(facts.jobs.pending===0 && facts.jobs.running===0 && facts.jobs.done===0 && facts.jobs.failed===0,"private project started library analysis");
            facts.reopening=reopening;facts.outputPath=renderExporter.outputPath;
            reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer { interval:200; repeat:true; running:smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>300) throw new Error("source disclosure timed out");smoke.step(); }
        catch(error) { smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
