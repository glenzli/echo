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
    property int auditionCycle: 0
    property int auditionWaitTicks: 0
    property real auditionVolume: 0.8
    function require(value, message) { if (!value) throw new Error(message); }
    function checkBatchEditing() {
        const before = JSON.stringify(assembly.document);
        const a = assembly.tracks[0].clips[0], b = assembly.tracks[1].clips[0];
        const undoCount = assembly.undoStack.length;
        assembly.selectClip(0, a.id, 0, false);
        assembly.selectClip(1, b.id, Qt.ControlModifier, false);
        require(assembly.selectionCount === 2, "multi-selection failed");
        assembly.moveClip(1, b.id, b.timelineStartMillis + 1500);
        require(assembly.tracks[0].clips[0].timelineStartMillis === a.timelineStartMillis + 1500, "group move lost alignment");
        require(assembly.selectedClipId === b.id, "group move changed the active inspector clip");
        require(assembly.undoStack.length === undoCount + 1, "group move created multiple history steps");
        assembly.undo(); require(JSON.stringify(assembly.document) === before && assembly.selectionCount === 2, "group undo failed");
        assembly.redo(); assembly.undo();
        assembly.duplicateSelectedClip(); require(assembly.totalClipCount() === 4 && assembly.selectionCount === 2, "batch duplicate failed");
        assembly.undo();
        assembly.playheadMillis = 2000;
        assembly.splitSelectedClip(); require(assembly.totalClipCount() === 4 && assembly.selectionCount === 4, "batch split failed");
        assembly.undo();
        assembly.mutate(next => { const c = next.tracks[1].clips[0]; c.timelineStartMillis = 2000; c.sourceStartMillis = 0; c.sourceEndMillis = 2000; });
        assembly.selectClip(1, b.id, 0, false);
        assembly.rippleDelete(true);
        require(assembly.tracks[0].clips.length === 2 && assembly.tracks[1].clips.length === 0, "global ripple did not preserve background tails");
        require(assembly.tracks[0].clips[1].timelineStartMillis === 2000 && assembly.tracks[0].clips[1].sourceStartMillis === 4000, "ripple source mapping failed");
        assembly.undo(); assembly.undo();
        require(JSON.stringify(assembly.document) === before, "batch editing changed originals after undo");
        facts.batchEditing = {selection: true, move: true, duplicate: true, split: true, ripple: true, undoRedo: true};
    }
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
            checkBatchEditing();
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
            assembly.stopPlayback();
            auditionVolume=player.volume;player.volume=0;
            stage=30;break;
        case 30:
            if(auditionCycle===24) {
                player.stop();player.volume=auditionVolume;
                facts.repeatedAuditions=auditionCycle;
                stage=6;break;
            }
            player.play(soundAssemblyController.previewPath);
            require(player.active && player.duration>0,"repeated audition failed to start");
            player.seek(100);auditionWaitTicks=0;stage=31;break;
        case 31:
            if(player.position<150) {
                require(++auditionWaitTicks<12,"device callback did not advance the audition");
                return;
            }
            player.togglePause();require(player.paused,"audition did not pause");
            player.togglePause();require(player.playing,"audition did not resume");
            ++auditionCycle;
            // Alternate explicit stop/replay with direct replacement while active.
            if(auditionCycle%2===0) player.stop();
            stage=30;break;
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
