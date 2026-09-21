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
    property int waveformPublications: 0
    Connections {
        target: assemblyWaveforms
        function onWaveformsChanged() { ++smoke.waveformPublications; }
    }
    function require(value, message) { if (!value) throw new Error(message); }
    function checkUnchangedEdits() {
        const document = assembly.document;
        const undo = assembly.undoStack.length, redo = assembly.redoStack.length;
        const dirty = assembly.dirty, active = player.active, publications = waveformPublications;
        assembly.setTrackValue(0, "gainCentibels", assembly.tracks[0].gainCentibels);
        assembly.setClipValue("timelineStartMillis", assembly.selectedClip.timelineStartMillis);
        assembly.setMasterValue("gainCentibels", assembly.document.master.gainCentibels);
        require(assembly.document === document, "unchanged edit rebuilt the document");
        require(assembly.undoStack.length === undo && assembly.redoStack.length === redo, "unchanged edit altered history");
        require(assembly.dirty === dirty && player.active === active, "unchanged edit dirtied the project or stopped playback");
        require(waveformPublications === publications, "unchanged edit republished source waveforms");
    }
    function checkEditingContext() {
        assembly.sourcesVisible = false;
        const selected = assembly.selectedClipId, playhead = assembly.playheadMillis;
        shell.chooseSource(0); shell.showMultitrack();
        require(!assembly.sourcesVisible, "mode switching reopened the source sidebar");
        require(assembly.selectedClipId === selected && assembly.playheadMillis === playhead, "mode switching lost editing context");
        checkUnchangedEdits();
        facts.editingContext = {sidebar: true, selection: true, playhead: true, unchangedEdits: true};
    }
    function checkLiveMix() {
        const original = assembly.authoredJson(assembly.document);
        const undoCount = assembly.undoStack.length, position = player.position;
        for (let i = 0; i < 100; ++i) assembly.previewTrackValue(0, "panPercent", i);
        require(assembly.undoStack.length === undoCount && assembly.authoredJson(assembly.document) === original,
                "transient mix audition changed the authored draft");
        assembly.setTrackValue(0, "gainCentibels", -500);
        require(player.playing && assembly.previewCurrent && assembly.undoStack.length === undoCount + 1,
                "live gain interrupted playback or broke history");
        assembly.undo(); assembly.redo();
        require(player.playing && assembly.previewCurrent, "mix undo/redo interrupted playback");
        assembly.setTrackValue(0, "panPercent", -80);
        assembly.setTrackValue(1, "muted", true);
        assembly.setTrackValue(0, "solo", true);
        assembly.setMasterValue("gainCentibels", -300);
        player.togglePause();
        assembly.setTrackValue(0, "panPercent", 75);
        require(player.paused && assembly.previewCurrent, "paused mix edit changed transport state");
        player.togglePause();
        require(assembly.saveRevision() && player.playing && assembly.previewCurrent, "mix save invalidated audition");
        require(player.position >= position, "live controls rewound playback");
        while (assembly.undoStack.length > undoCount) assembly.undo();
        require(assembly.authoredJson(assembly.document) === original && assembly.previewCurrent, "mix undo failed to restore draft");
        require(assembly.saveRevision(), "restored mix failed to save");
        for (const kind of ["source", "clip", "limiter", "invalid"]) {
            const next = assembly.clone(assembly.document);
            if (kind === "source") next.clipSources[0].path += ".wrong";
            if (kind === "clip") next.tracks[0].clips[0].timelineStartMillis += 50;
            if (kind === "limiter") next.master.limiterEnabled = !next.master.limiterEnabled;
            if (kind === "invalid") next.tracks[0].gainCentibels = 65536;
            require(!soundAssemblyController.updatePreviewMix(next, true), "unsafe mix update admitted: " + kind);
        }
        require(player.playing && assembly.previewCurrent, "rejected control update disturbed audition");
        facts.liveMix = {trackControls:true, masterGain:true, transientDrag:true, undoRedo:true, pause:true, save:true, topologyRejection:true};
    }
    function checkMarkers() {
        const audio = JSON.stringify(assembly.tracks);
        assembly.playheadMillis = 1200;
        assembly.addMarker(false);
        const point = assembly.selectedMarkerId;
        assembly.patchMarker(point, "name", "Rain entrance");
        assembly.addMarker(true);
        const range = assembly.selectedMarkerId;
        assembly.patchMarker(range, "name", "Listen detail");
        assembly.patchMarker(range, "startMillis", 1000);
        assembly.patchMarker(range, "endMillis", 2500);
        assembly.deleteMarker(point); assembly.undo();
        require(assembly.markers.length === 2 && JSON.stringify(assembly.tracks) === audio, "markers altered audio or failed undo");
        assembly.playheadMillis = 0; assembly.navigateMarker(1);
        require(assembly.playheadMillis === 1000, "next marker did not navigate");
        assembly.navigateMarker(1); require(assembly.playheadMillis === 1200, "next point marker did not navigate");
        assembly.navigateMarker(-1); require(assembly.playheadMillis === 1000, "previous marker did not navigate");
        facts.markers = {point: point, range: range, count: 2, undo: true, navigation: true};
    }
    function checkWaveforms() {
        const counts = [];
        for (const levels of Object.values(assemblyWaveforms.waveforms)) {
            require(levels.length > 1 && levels[0].mins.length <= 8192, "source waveform pyramid missing or unbounded");
            counts.push(levels.map(level => level.mins.length));
            for (let l = 1; l < levels.length; ++l) {
                const fine = levels[l - 1], coarse = levels[l];
                require(coarse.mins.length === Math.ceil(fine.mins.length / 2), "waveform pyramid dropped the tail");
                for (let i = 0; i < coarse.mins.length; ++i) {
                    const tail = Math.min(i * 2 + 1, fine.mins.length - 1);
                    require(coarse.mins[i] === Math.min(fine.mins[i * 2], fine.mins[tail]) && coarse.maxs[i] === Math.max(fine.maxs[i * 2], fine.maxs[tail]), "waveform pyramid lost a transient");
                }
            }
        }
        const publications = waveformPublications;
        assembly.setTrackValue(0, "gainCentibels", assembly.tracks[0].gainCentibels - 10);
        assembly.undo();
        require(waveformPublications === publications, "mix-only edits republished unchanged waveforms");
        facts.waveformBuckets = counts;
        facts.waveformReuse = true;
    }
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
                require(assembly.markers.length === 2 && assembly.markers.some(item => item.name === "Listen detail" && item.endMillis === 2500), "portable markers lost");
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
            checkEditingContext();
            assembly.mutate(next=>{next.name='Independent arrangement';next.tracks[0].clips[0].fadeInMillis=120;next.tracks[1].clips[0].timelineStartMillis=500;});
            assembly.selectClip(1, assembly.tracks[1].clips[0].id);
            stage=20;break;
        case 20:
            if(Object.values(assemblyWaveforms.waveforms).filter(levels=>levels.length>0).length!==2) return;
            checkWaveforms();
            assembly.ducking.generate();stage=21;break;
        case 21:
            if(assembly.ducking.running) return;
            require(assembly.ducking.candidates.length>0,assembly.ducking.message || "ducking generated no candidate");
            assembly.ducking.applyRequested(assembly.ducking.candidates);
            require(assembly.selectedClip.gainEnvelope.points.length>=2,"ducking curve missing");
            facts.duckingEnvelope=assembly.selectedClip.gainEnvelope;
            assembly.undo();require(!assembly.selectedClip.gainEnvelope,"ducking undo failed");
            assembly.redo();require(assembly.selectedClip.gainEnvelope.enabled,"ducking redo failed");
            checkMarkers();
            require(shell.flushDrafts(),"assembly failed to save");
            independentEditor.saveProject('file://'+fixtureRoot+'/portable.echo');stage=4;break;
        case 4:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText,independentEditor.errorText);
            assembly.previewMarker(facts.markers.range);++stage;break;
        case 5:
            if(soundAssemblyController.running) return;
            require(soundAssemblyController.hasPreview,soundAssemblyController.errorText || "preview failed");
            facts.previewDuration=player.duration;
            require(Math.abs(player.duration-(assembly.previewRangeEnd-assembly.previewRangeStart))<2,"range preview duration mismatch");
            checkUnchangedEdits();
            facts.unchangedEditsPreservePlayback = true;
            checkLiveMix();
            const marker = assembly.markers.find(item => item.id === facts.markers.range);
            assembly.patchMarker(marker.id, "name", "Temporary name");
            require(player.playing && assembly.previewCurrent, "marker rename interrupted playback");
            assembly.undo();
            assembly.patchMarker(marker.id, "endMillis", marker.endMillis + 250);
            require(!player.active && !assembly.previewCurrent, "named-range boundary retained a stale preview window");
            assembly.undo();
            require(assembly.previewCurrent, "range undo did not recover the correct preview identity");
            facts.markers.previewDuration = facts.previewDuration;
            facts.markers.rangeInvalidation = true;
            assembly.stopPlayback();
            auditionVolume=player.volume;player.volume=0;
            stage=30;break;
        case 30:
            if(auditionCycle===24) {
                player.stop();player.volume=auditionVolume;
                facts.repeatedAuditions=auditionCycle;
                stage=6;break;
            }
            require(soundAssemblyController.playPreview(),"streaming preview could not restart");
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
