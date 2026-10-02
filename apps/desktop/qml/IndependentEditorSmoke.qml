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
    property int originalClipGain: 0
    property string conflictClipId: ""
    property string conflictProjectName: ""
    property var singleEditBaseline: ({})
    property int singleEditHistoryIndex: 0
    property int singlePausedPosition: 0
    property string singlePlaybackKey: ""
    property real singlePlaybackVolume: 0
    Connections {
        target: assemblyWaveforms
        function onWaveformsChanged() { ++smoke.waveformPublications; }
    }
    function require(value, message) { if (!value) throw new Error(message); }
    function itemNamed(root, name) {
        if (root.objectName === name) return root;
        for (const child of root.children || []) {
            const found = itemNamed(child, name);
            if (found) return found;
        }
        return null;
    }
    function checkRevisionConflicts() {
        require(assembly.saveRevision(), "revision regression baseline failed");
        const original = assembly.clone(assembly.document);
        assembly.setTrackValue(0, "gainCentibels", original.tracks[0].gainCentibels - 10);
        const edited = assembly.saveRevision();
        require(edited, "edited revision failed to save");
        assembly.undo();
        require(assembly.document.revisionId === edited.revisionId && assembly.dirty, "undo restored an obsolete save base");
        const undone = assembly.saveRevision();
        require(undone, "save after undo conflicted with its own prior save");
        assembly.redo();
        require(assembly.document.revisionId === undone.revisionId && assembly.saveRevision(), "redo did not retain the current save base");
        const remote = assembly.clone(assembly.document);
        remote.name = "Saved by another editor";
        const winner = backend.saveSoundAssembly(remote);
        require(winner && !winner.error, "second writer did not publish");
        assembly.setMasterValue("gainCentibels", original.master.gainCentibels - 25);
        const draft = JSON.stringify(assembly.document), base = assembly.savedRevisionBase.revisionId;
        const undoCount = assembly.undoStack.length, redoCount = assembly.redoStack.length;
        require(!assembly.saveRevision() && assembly.revisionConflict, "stale draft overwrote another revision");
        require(shell.draftConflict && !shell.draftsRecoverable, "conflicting assembly draft was advertised as recoverable");
        // This controller rejects the URL synchronously, without accessing a file.
        independentEditor.saveProject("invalid:synthetic-local-destination");
        require(independentEditor.errorText.length > 0 && !independentEditor.busy,
                "nonlocal project destination was not rejected immediately");
        const previousNotice = shell.notice;
        shell.notice = "Earlier operation completed";
        const conflictStatus = itemNamed(shell.contentItem, "independentEditorStatus");
        require(conflictStatus && conflictStatus.text === shell.recoveryStatusText,
                "an earlier notice concealed the unsaved conflict status");
        shell.notice = previousNotice;
        require(JSON.stringify(assembly.document) === draft && assembly.dirty && assembly.savedRevisionBase.revisionId === base,
                "conflict discarded the draft or advanced its save base");
        require(assembly.undoStack.length === undoCount && assembly.redoStack.length === redoCount, "conflict changed undo history");
        assembly.reloadDialog.open(); assembly.reloadDialog.reject();
        require(JSON.stringify(assembly.document) === draft && assembly.revisionConflict, "cancel reload discarded the draft");
        require(!assembly.saveRevision(), "repeated stale save bypassed the precondition");
        require(backend.soundAssembly(assembly.document.id).revisionId === winner.revisionId, "failed save published a revision");
        assembly.reloadDialog.open(); assembly.reloadDialog.accept();
        require(!assembly.dirty && !assembly.revisionConflict && assembly.document.revisionId === winner.revisionId,
                "confirmed reload did not adopt the current revision");
        require(!assembly.canUndo && !assembly.canRedo, "confirmed reload retained obsolete history");
        require(!shell.draftConflict && shell.draftsRecoverable, "confirmed reload retained a false recovery warning");
        assembly.mutate(next => { next.name = original.name; next.master = assembly.clone(original.master); next.tracks = assembly.clone(original.tracks); });
        require(assembly.saveRevision(), "new edits after conflict recovery could not save");
        facts.revisionConflicts = {saveUndoSave:true, saveRedoSave:true, draftRetained:true, historyRetained:true,
                                  cancelReload:true, repeatedStaleSaveRejected:true, explicitReload:true, freshSave:true};
    }
    function checkPendingInputs() {
        assembly.markersVisible=false; assembly.inspectorVisible=true;
        const first=assembly.tracks[0].clips[0].id, second=assembly.tracks[1].clips[0].id;
        assembly.selectClip(0,first,0,false);
        require(shell.flushDrafts(),"pending input baseline could not save");
        const initial=assembly.selectedClip.sourceEndMillis, undoCount=assembly.undoStack.length;
        const field=itemNamed(assembly,"inspector-sourceEndMillis");
        require(field!==null,"timing field not instantiated");
        const typed=Math.min(initial-500,3500);
        field.contentItem.forceActiveFocus();
        field.contentItem.text=field.textFromValue(typed,field.locale);
        require(shell.checkpointDrafts(),"recovery checkpoint failed");
        require(field.contentItem.activeFocus && assembly.selectedClip.sourceEndMillis===initial && assembly.undoStack.length===undoCount,
                "recovery committed an unfinished input or stole focus");
        require(shell.flushDrafts(),"focused timing failed to save");
        require(assembly.selectedClip.sourceEndMillis===typed && assembly.undoStack.length===undoCount+1,
                "save omitted pending seconds or recorded multiple edits");
        const saved=backend.soundAssembly(assembly.document.id);
        require(saved.tracks[0].clips[0].sourceEndMillis===typed,"saved revision retained stale seconds");
        assembly.undo(); require(assembly.selectedClip.sourceEndMillis===initial,"typed timing undo failed");
        assembly.redo(); require(assembly.selectedClip.sourceEndMillis===typed,"typed timing redo failed");
        assembly.undo();
        const otherEnd=assembly.tracks[1].clips[0].sourceEndMillis;
        field.contentItem.forceActiveFocus(); field.contentItem.text=field.textFromValue(typed,field.locale);
        assembly.selectClip(1,second,0,false);
        require(assembly.tracks[0].clips[0].sourceEndMillis===typed && assembly.tracks[1].clips[0].sourceEndMillis===otherEnd,
                "selection lost input or applied it to the next clip");
        assembly.undo(); require(assembly.tracks[0].clips[0].sourceEndMillis===initial,"selection commit undo failed");
        require(shell.flushDrafts(),"pending input baseline restoration failed");
        facts.pendingInput={save:true,persisted:true,undoRedo:true,selectionTarget:true,recoveryPreservesTyping:true};
    }
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
        const dirtyBefore=assembly.dirty;
        assembly.exactTimeDialog.present(500,500);assembly.exactTimeDialog.apply();
        require(assembly.playheadMillis===500 && assembly.dirty===dirtyBefore,"exact project navigation changed the document");
        assembly.seekTo(playhead);
        facts.exactProjectTime=true;
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
        const before = assembly.authoredJson(assembly.document);
        const a = assembly.tracks[0].clips[0], b = assembly.tracks[1].clips[0];
        const undoCount = assembly.undoStack.length;
        assembly.selectClip(0, a.id, 0, false);
        assembly.selectClip(1, b.id, Qt.ControlModifier, false);
        require(assembly.selectionCount === 2, "multi-selection failed");
        assembly.moveClip(1, b.id, b.timelineStartMillis + 1500);
        require(assembly.tracks[0].clips[0].timelineStartMillis === a.timelineStartMillis + 1500, "group move lost alignment");
        require(assembly.selectedClipId === b.id, "group move changed the active inspector clip");
        require(assembly.undoStack.length === undoCount + 1, "group move created multiple history steps");
        assembly.undo(); require(assembly.authoredJson(assembly.document) === before && assembly.selectionCount === 2, "group undo failed");
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
        require(assembly.authoredJson(assembly.document) === before, "batch editing changed originals after undo");
        facts.batchEditing = {selection: true, move: true, duplicate: true, split: true, ripple: true, undoRedo: true};
    }
    function step() {
        switch(stage) {
        case 0:
            if (independentEditor.busy || shell.assets.length < 2) return;
            require(backend.independentEditing, "not an independent session");
            player.volume = 0; // Synthetic device callbacks are exercised silently.
            const jobs=backend.jobStats();
            require(jobs.pending===0 && jobs.running===0 && jobs.done===0 && jobs.failed===0, "library jobs started");
            facts.assets=shell.assets.map(value=>({id:value.id,path:value.path,gain:value.gainCentibels}));
            if(reopening) {
                require(!shell.projectDirty && !editor.dirty && !assembly.dirty, "saved project opened dirty");
                backend.refresh();
                require(!shell.projectDirty, "read-only refresh dirtied the saved project");
                shell.chooseSource(0); shell.chooseSource(1); shell.showMultitrack();
                require(!shell.projectDirty && !editor.dirty, "source switching dirtied the saved project");
                facts.cleanProjectLifecycle = true;
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
            singleEditBaseline = editor.adjustment.snapshot();
            singleEditHistoryIndex = editor.adjustment._historyIndex;
            singlePlaybackVolume = player.volume; player.volume = 0;
            editor.playFrom(1750);
            require(player.playing, "single-source audition did not start");
            player.togglePause();
            singlePausedPosition = player.position;
            singlePlaybackKey = editor.loadedAdjustmentKey;
            editor.setOriginalAudition(true);
            require(player.paused && editor.loadedAdjustmentKey === singlePlaybackKey,
                    "paused A/B restarted the device");
            stage=40;break;
        case 40:
            require(player.paused && player.position === singlePausedPosition,
                    "paused comparison advanced before explicit Play");
            editor.setOriginalAudition(false);
            require(player.paused && editor.loadedAdjustmentKey === singlePlaybackKey,
                    "switching back restarted paused playback");
            editor.setOriginalAudition(true); editor.togglePlayback();
            require(player.playing && editor.loadedAdjustmentKey === editor.adjustmentKey() &&
                    Math.abs(player.position - singlePausedPosition) < 100,
                    "explicit Play did not resume the chosen comparison at the source position");
            editor.setOriginalAudition(false);
            require(player.playing && editor.loadedAdjustmentKey === editor.adjustmentKey() &&
                    Math.abs(player.position - singlePausedPosition) < 100,
                    "playing comparison changed the source position or stopped playback");
            player.stop(); editor.setOriginalAudition(true); editor.setOriginalAudition(false);
            require(!player.active, "stopped comparison unexpectedly started playback");
            player.volume = singlePlaybackVolume;
            require(editor.adjustment.sameSnapshot(editor.adjustment.snapshot(), singleEditBaseline),
                    "comparison changed the editable draft");
            facts.singleAudition = {pausedBothDirections:true, pausedPositionStable:true,
                                    explicitResume:true, playingSwitch:true, stoppedSwitch:true, draftUnchanged:true};
            editor.adjustment.beginGesture();
            editor.adjustment.setTrimRange(500, 3500);
            editor.adjustment.setFades(250, 500);
            editor.adjustment.setGain(-1200);
            editor.adjustment.endGesture();
            require(editor.dirty, "export scenario did not have unsaved edits");
            editor.openExport();
            editor.exportDialog.destination = 'file://' + fixtureRoot + '/edited.wav';
            const exportButton = itemNamed(editor.exportDialog.contentItem, "startSoundExport");
            require(exportButton && exportButton.enabled, "dirty export action was unavailable");
            exportButton.clicked();
            require(!editor.dirty && !editor.exportDialog.saveErrorText,
                    editor.exportDialog.saveErrorText || "one-click export did not save the draft");
            facts.editedExport = {assetId:editor.asset.id, adjustmentRevision:Number(editor.asset.adjustmentRevision),
                                  trimStartMillis:500, trimEndMillis:3500, fadeInMillis:250, fadeOutMillis:500,
                                  gainCentibels:-1200, oneExplicitAction:true};
            require(facts.editedExport.adjustmentRevision > 0, "saved export lacks a revision identity");
            stage=41;break;
        case 41:
            if (renderExporter.running) return;
            require(renderExporter.hasResult && editor.exportDialog.outputAvailable,
                    renderExporter.errorText || "dirty save-and-export failed");
            const created = itemNamed(editor.exportDialog.contentItem, "exportCreatedPath");
            require(created && created.visible && created.text.indexOf(renderExporter.outputPath) >= 0,
                    "completed export did not display the created file");
            facts.editedExport.outputPath = renderExporter.outputPath;
            shell.width = 1240; shell.height = 720;
            stage=42;break;
        case 42:
            const delivery = editor.exportDialog;
            require(delivery.x >= 0 && delivery.y >= 0 &&
                    delivery.x + delivery.width <= delivery.parent.width + 1 &&
                    delivery.y + delivery.height <= delivery.parent.height + 1,
                    "export receipt or actions do not fit the minimum editing window");
            facts.editedExport.minimumWindow = {width:shell.width, height:shell.height,
                dialogWidth:delivery.width, dialogHeight:delivery.height, availableHeight:delivery.parent.height};
            editor.exportDialog.close();
            shell.width = 1500; shell.height = 900;
            editor.undo();
            require(editor.adjustment._historyIndex === singleEditHistoryIndex &&
                    editor.adjustment.sameSnapshot(editor.adjustment.snapshot(), singleEditBaseline),
                    "save-and-export lost the edit history");
            require(shell.flushDrafts(), "original single-source draft could not be restored");
            facts.editedExport.undoAfterSave = true;
            editor.debugExport('file://'+fixtureRoot+'/single.wav');stage=3;break;
        case 3:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,renderExporter.errorText || "single-source export failed");
            facts.singleExport=renderExporter.outputPath;
            shell.showMultitrack();require(assembly.hasDocument,"assembly creation failed");
            stage=22;break;
        case 22:
            checkPendingInputs();
            checkRevisionConflicts();
            checkBatchEditing();
            checkEditingContext();
            assembly.mutate(next=>{next.name='Independent arrangement';next.tracks[0].clips[0].fadeInMillis=120;next.tracks[1].clips[0].timelineStartMillis=500;});
            assembly.selectClip(1, assembly.tracks[1].clips[0].id);
            require(assembly.saveRevision(), "clip conflict baseline could not save");
            conflictClipId = assembly.selectedClipId;
            conflictProjectName = assembly.document.name;
            assembly.openClipEditor();
            stage=23;break;
        case 23:
            if (!editor.editingProjectClip || !editor.hasAsset) return;
            originalClipGain = editor.adjustment.gainCentibels;
            editor.adjustment.setGain(originalClipGain - 25);
            const clipDraft = JSON.stringify(editor.adjustment.snapshot());
            const clipHistory = JSON.stringify(editor.adjustment._history);
            const clipHistoryIndex = editor.adjustment._historyIndex;
            require(editor.canUndo, "clip draft did not record undo history");
            const remoteProject = assembly.clone(assembly.document);
            remoteProject.name = "Changed while editing the clip";
            const clipWinner = backend.saveSoundAssembly(remoteProject);
            require(clipWinner && !clipWinner.error, "remote clip project could not save");
            editor.save();
            require(editor.dirty && editor.projectRevisionConflict && JSON.stringify(editor.adjustment.snapshot()) === clipDraft,
                    "clip conflict lost its draft or was not exposed");
            require(shell.draftConflict && !shell.draftsRecoverable, "conflicting clip draft was advertised as recoverable");
            editor.projectReloadDialog.open(); editor.projectReloadDialog.reject();
            require(editor.dirty && JSON.stringify(editor.adjustment.snapshot()) === clipDraft, "cancel clip reload discarded edits");
            require(JSON.stringify(editor.adjustment._history) === clipHistory && editor.adjustment._historyIndex === clipHistoryIndex,
                    "clip conflict or cancel altered undo history");
            editor.returnToProjectRequested();
            require(!shell.multitrack && editor.dirty, "normal navigation silently discarded conflicting clip edits");
            editor.projectReloadDialog.open(); editor.projectReloadDialog.accept();
            require(shell.multitrack && !editor.projectRevisionConflict && !editor.dirty && assembly.document.revisionId === clipWinner.revisionId,
                    "confirmed clip recovery did not return to the latest project");
            require(!editor.canUndo && !editor.canRedo, "confirmed clip reload retained discarded history");
            assembly.selectClip(1, conflictClipId, 0, false);
            assembly.openClipEditor();
            stage=24;break;
        case 24:
            if (!editor.editingProjectClip || !editor.hasAsset) return;
            require(!editor.canUndo && !editor.canRedo, "reopening the clip revived discarded history");
            editor.adjustment.setGain(originalClipGain - 25);
            editor.save();
            require(!editor.dirty && !editor.projectRevisionConflict, "clip edits after reload could not save");
            editor.adjustment.setGain(originalClipGain);
            editor.save();
            require(!editor.dirty, "clip gain restoration could not save");
            editor.returnToProjectRequested();
            require(shell.multitrack, "return after recovered clip save failed");
            assembly.mutate(next => next.name = conflictProjectName);
            require(assembly.saveRevision(), "recovered project title restoration failed");
            facts.clipRevisionConflicts = {draftRetained:true, historyRetainedOnCancel:true, discardedHistoryCleared:true,
                                           cancelReload:true, navigationPreservesDraft:true,
                                           explicitReload:true, returnedToLatestProject:true, freshClipSave:true};
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
            if (!reopening) {
                const archivedRemote = assembly.clone(assembly.document);
                archivedRemote.name = "Remote version before archival";
                const archiveWinner = backend.saveSoundAssembly(archivedRemote);
                require(archiveWinner && !archiveWinner.error, "archive conflict setup failed");
                assembly.setMasterValue("gainCentibels", assembly.document.master.gainCentibels - 1);
                require(!assembly.saveRevision() && assembly.revisionConflict, "archive setup did not conflict");
                assembly.archiveCurrentAssembly();
                require(!assembly.hasDocument && !assembly.revisionConflict && !assembly.errorText &&
                        Object.keys(assembly.savedRevisionBase).length === 0 && !assembly.canUndo && !assembly.canRedo,
                        "archiving the last project retained a stale conflict or undo base");
                facts.archiveClearsConflict = true;
                const addSound = itemNamed(assembly, "assemblyAddSound");
                require(addSound && addSound.enabled, "empty independent project disabled Add sound");
                addSound.clicked();
                require(assembly.sourcesVisible, "Add sound did not open the project sources");
                const sources = itemNamed(assembly, "assemblySourceBrowser");
                require(sources && sources.visible && sources.filteredAssets.length > 0,
                        "empty project did not expose its retained sources");
                const retained = sources.filteredAssets.find(asset => !asset.assemblyId && asset.pathStatus === "present");
                require(retained, "archiving the assembly lost usable project sources");
                sources.addSource(retained);
                require(assembly.hasDocument && assembly.document.id !== archivedRemote.id &&
                        assembly.totalClipCount() === 1 && assembly.tracks[0].clips[0].assetId === retained.id,
                        "adding a retained source did not start a new arrangement");
                require(assembly.checkpointRevision(), "new arrangement could not save after archival");
                const restarted = backend.soundAssembly(assembly.document.id);
                require(restarted && !restarted.error && restarted.tracks[0].clips[0].assetId === retained.id,
                        "new arrangement was not persisted through the normal service");
                facts.emptyAssemblyRecovery = {addSoundEnabled:true, sourcesRetained:true,
                                               addSourceCreatesArrangement:true, checkpointSaved:true};
            }
            reportJson=JSON.stringify({ok:true,reopening:reopening,facts:facts});break;
        }
    }
    Timer { interval: 250; repeat: true; running: smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>240) throw new Error('independent editor workflow timed out'); smoke.step(); }
        catch(error) { player.stop(); smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
