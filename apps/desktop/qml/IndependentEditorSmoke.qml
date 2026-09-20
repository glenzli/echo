//! Opt-in packaged integration: private source admission, editing, portable save,
//! multitrack export and the same rendering after reopening a moved project.
import QtQuick
Item {
    id: smoke
    required property var shell
    required property var editor
    required property var assembly
    required property string fixtureRoot
    required property bool reopening
    property int stage: 0
    property int ticks: 0
    property string reportJson: ""
    property var facts: ({})
    function require(value, message) { if (!value) throw new Error(message); }
    function step() {
        switch(stage) {
        case 0:
            if (independentEditor.busy || shell.assets.length < 2) return;
            require(backend.independentEditing, "not an independent session");
            const jobs=backend.jobStats();
            require(jobs.pending===0 && jobs.running===0 && jobs.done===0 && jobs.failed===0, "library jobs started");
            facts.assets=shell.assets.map(value=>({id:value.id,path:value.path,gain:value.gainCentibels}));
            if(reopening) {
                require(assembly.hasDocument && assembly.tracks.length===2,"portable assembly missing");
                require(shell.assets.some(value=>value.gainCentibels===-600),"source adjustment lost");
                require(assembly.tracks[1].clips[0].gainEnvelope.enabled,"portable envelope lost");
                stage=6; return;
            }
            shell.chooseSource(0); ++stage; break;
        case 1:
            if(!editor.hasAsset) return;
            editor.spectralFocus = true;
            editor.adjustment.setGain(-600);
            require(shell.flushDrafts(),"single-source draft failed to save");
            require(editor.canUndo,"checkpoint cleared undo history");
            editor.undo();require(editor.adjustment.gainCentibels===0,"undo after checkpoint failed");
            editor.redo();require(editor.adjustment.gainCentibels===-600,"redo after checkpoint failed");
            require(shell.flushDrafts(),"redo checkpoint failed");
            independentEditor.saveProject('file://'+fixtureRoot+'/single.echo'); ++stage;break;
        case 2:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText, independentEditor.errorText);
            require(!spectrogramPreview.errorText, spectrogramPreview.errorText);
            if(!spectrogramPreview.imageUrl) return;
            facts.spectrogramReady=true;
            editor.debugExport('file://'+fixtureRoot+'/single.wav');++stage;break;
        case 3:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,renderExporter.errorText || "single-source export failed");
            facts.singleExport=renderExporter.outputPath;
            shell.showMultitrack();require(assembly.hasDocument,"assembly creation failed");
            assembly.mutate(next=>{next.name='Independent arrangement';next.tracks[0].clips[0].fadeInMillis=120;next.tracks[1].clips[0].timelineStartMillis=500;});
            assembly.selectClip(1, assembly.tracks[1].clips[0].id);
            stage=20;break;
        case 20:
            if(Object.values(assemblyWaveforms.waveforms).filter(levels=>levels.length>0).length!==2) return;
            assembly.ducking.generate();stage=21;break;
        case 21:
            if(assembly.ducking.running) return;
            require(assembly.ducking.candidates.length>0,assembly.ducking.message || "ducking generated no candidate");
            assembly.ducking.applyRequested(assembly.ducking.candidates);
            require(assembly.selectedClip.gainEnvelope.points.length>=2,"ducking curve missing");
            facts.duckingEnvelope=assembly.selectedClip.gainEnvelope;
            assembly.undo();require(!assembly.selectedClip.gainEnvelope,"ducking undo failed");
            assembly.redo();require(assembly.selectedClip.gainEnvelope.enabled,"ducking redo failed");
            require(shell.flushDrafts(),"assembly failed to save");
            independentEditor.saveProject('file://'+fixtureRoot+'/portable.echo');stage=4;break;
        case 4:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText,independentEditor.errorText);
            assembly.previewSelection=true;
            assembly.preview();++stage;break;
        case 5:
            if(soundAssemblyController.running) return;
            require(soundAssemblyController.hasPreview,soundAssemblyController.errorText || "preview failed");
            facts.previewDuration=player.duration;
            require(Math.abs(player.duration-(assembly.previewRangeEnd-assembly.previewRangeStart))<2,"range preview duration mismatch");
            assembly.stopPlayback();stage=6;break;
        case 6:
            soundAssemblyController.exportAssembly(assembly.document,'file://'+fixtureRoot+(reopening?'/reopened.wav':'/mix.wav'));++stage;break;
        case 7:
            if(soundAssemblyController.running) return;
            require(soundAssemblyController.hasResult,soundAssemblyController.errorText || "mix export failed");
            if(Object.keys(assemblyWaveforms.waveforms).length!==2) return;
            facts.reusedSources=soundAssemblyController.reusedSourceCount;
            if(!reopening) require(facts.reusedSources===2,"unchanged source preparation was not reused");
            facts.mixExport=soundAssemblyController.outputPath;
            facts.timelineScale=assembly.pixelsPerSecond;
            require(assembly.durationMillis*assembly.pixelsPerSecond/1000>200,"timeline was fitted before layout");
            require(Object.keys(assemblyWaveforms.waveforms).length===2,"project waveforms missing");
            require(Object.values(assemblyWaveforms.waveforms).every(levels=>levels.length>0),"empty project waveform");
            facts.tracks=assembly.tracks.length;facts.assembly=assembly.document;
            require(backend.listRoots().length===0,"a scan root was registered");
            reportJson=JSON.stringify({ok:true,reopening:reopening,facts:facts});break;
        }
    }
    Timer { interval: 250; repeat: true; running: smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>240) throw new Error('independent editor workflow timed out'); smoke.step(); }
        catch(error) { player.stop(); smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
