//! Opt-in native contract using attributed audio and Infer Runtime speech fixtures.
import QtQuick

Item {
    id: smoke
    required property var shell
    required property var assembly
    required property string fixtureRoot
    property bool stress: false
    property real renderStarted: 0
    property real lastHeartbeat: 0
    property var heartbeatGaps: []
    property string stressDocument: ""
    property int stage: 0
    property string reportJson: ""
    property var facts: ({})
    property string rainId: ""
    property string voiceId: ""
    property string musicId: ""
    property int ticks: 0
    property string originalLanguage: ""
    property int originalAppearance: -1
    property real headerX: 0

    function require(condition: bool, message: string): void {
        if (!condition)
            throw new Error(message);
    }
    function itemNamed(root: var, name: string): var {
        if (root.objectName === name)
            return root;
        for (const child of root.children || []) {
            const found = itemNamed(child, name);
            if (found)
                return found;
        }
        return null;
    }
    function restorePreferences(): void {
        if (originalLanguage)
            uiPrefs.languageMode = originalLanguage;
        if (originalAppearance >= 0)
            uiPrefs.mode = originalAppearance;
    }
    function step(): void {
        const assets = backend.listAssets();
        require(!soundAssemblyController.errorText, soundAssemblyController.errorText);
        switch (stage) {
        case 0:
            {
                const rain = assets.find(asset => asset.path.endsWith("Rain in the gutter.mp3"));
                if (!rain)
                    return;
                originalLanguage = uiPrefs.languageMode;
                originalAppearance = uiPrefs.mode;
                rainId = rain.id;
                shell.createSoundAssembly([rainId], "sequence");
                require(assembly.hasDocument, "project creation failed");
                require(!backend.importMaterial("file://" + fixtureRoot + "/After the rain — synthetic narration.wav", assembly.document.id, true, "voice"), "voice import rejected");
                require(!backend.importMaterial("file://" + fixtureRoot + "/Forest ambience music.mp3", assembly.document.id, true, "music"), "music import rejected");
                stage = 1;
                break;
            }
        case 1:
            {
                const voice = assets.find(asset => asset.path.endsWith("After the rain — synthetic narration.wav") && asset.inMaterials);
                const music = assets.find(asset => asset.path.endsWith("Forest ambience music.mp3") && asset.inMaterials);
                if (!voice || !music)
                    return;
                voiceId = voice.id;
                musicId = music.id;
                assembly.refreshAssemblies();
                assembly.addTrack();
                assembly.addLibraryAsset(voice, "material");
                assembly.addTrack();
                assembly.addLibraryAsset(music, "material");
                assembly.mutate(next => {
                    next.name = "雨后 · 多轨编辑演示";
                    next.tracks[0].name = "屋檐雨声 · CC0";
                    next.tracks[0].gainCentibels = -1400;
                    const first = next.tracks[0].clips[0];
                    first.sourceEndMillis = 10000;
                    first.fadeInMillis = 700;
                    const second = JSON.parse(JSON.stringify(first));
                    second.id = backend.newAssemblyObjectId();
                    second.sourceStartMillis = 8000;
                    second.sourceEndMillis = 18000;
                    second.timelineStartMillis = 8000;
                    second.fadeInMillis = 0;
                    second.fadeOutMillis = 1200;
                    next.tracks[0].clips.push(second);
                    next.tracks[1].name = "旁白 · AI 生成";
                    next.tracks[1].clips[0].timelineStartMillis = 2000;
                    next.tracks[1].clips[0].fadeInMillis = 80;
                    next.tracks[1].clips[0].fadeOutMillis = 150;
                    next.tracks[2].name = "氛围配乐 · CC0";
                    next.tracks[2].gainCentibels = 1100;
                    next.tracks[2].clips[0].gainCentibels = 1000;
                    next.tracks[2].clips[0].sourceEndMillis = 18000;
                    next.tracks[2].clips[0].fadeInMillis = 2400;
                    next.tracks[2].clips[0].fadeOutMillis = 3200;
                });
                assembly.selectClip(0, assembly.tracks[0].clips[0].id);
                require(assembly.crossfadeCandidate().duration === 2000, "overlap candidate incorrect");
                const undoCount = assembly.undoStack.length;
                assembly.crossfadeSelected();
                require(assembly.undoStack.length === undoCount + 1, "crossfade did not coalesce undo");
                require(assembly.tracks[0].clips[0].fadeOutMillis === 2000 && assembly.tracks[0].clips[1].fadeInMillis === 2000, "crossfade missing an endpoint");
                assembly.selectClip(1, assembly.tracks[1].clips[0].id);
                const before = JSON.stringify(assembly.document.tracks);
                assembly.duplicateSelectedClip();
                require(assembly.tracks[1].clips[1].timelineStartMillis === 14240, "duplicate was not placed after clip");
                assembly.selectClip(1, assembly.tracks[1].clips[0].id);
                assembly.rippleDelete();
                require(assembly.tracks[1].clips[0].timelineStartMillis === 2000, "ripple gap did not close");
                assembly.undo();
                assembly.undo();
                require(JSON.stringify(assembly.document.tracks) === before, "undo failed to recover source pins");
                require(assembly.saveRevision() !== null, "edited project failed to save");
                assembly.sourcesVisible = false;
                assembly.inspectorVisible = true;
                assembly.fitProject();
                assembly.seekTo(7000);
                stage = 2;
                break;
            }
        case 2:
            if (![rainId, voiceId, musicId].every(id => assemblyWaveforms.waveforms[id] && assemblyWaveforms.waveforms[id].length))
                return;
            assembly.preview();
            stage = 3;
            break;
        case 3:
            if (soundAssemblyController.running)
                return;
            require(assembly.previewCurrent && player.playing, "preview did not bind to current document");
            require(player.position >= 6900 && player.position < 9000, "preview ignored the playhead position");
            assembly.setTrackValue(2, "gainCentibels", 1200);
            require(!player.active, "editing did not stop stale playback");
            require(assembly.saveRevision() !== null && !assembly.previewCurrent, "saving made stale preview valid");
            assembly.togglePlayback();
            stage = 4;
            break;
        case 4:
            if (soundAssemblyController.running)
                return;
            require(assembly.previewCurrent, "new preview never became current");
            assembly.stopPlayback();
            assembly.seekTo(8500);
            assembly.selectClip(0, assembly.tracks[0].clips[1].id);
            assembly.keepMemory();
            stage = 5;
            break;
        case 5:
            {
                if (soundAssemblyController.running)
                    return;
                const memory = assets.find(asset => asset.assemblyId === assembly.document.id && asset.inMemory);
                if (!memory)
                    return;
                const evidence = JSON.parse(memory.provenanceJson);
                require(evidence.sources.length === 4, "mix did not preserve every clip reference");
                require(evidence.sources.some(source => source.assetId === voiceId && source.sourceRole === "material"), "synthetic voice lost material role");
                facts = {
                    projectId: assembly.document.id,
                    revisionId: assembly.document.revisionId,
                    outputPath: memory.path,
                    durationMillis: assembly.durationMillis,
                    waveforms: 3,
                    crossfadeMillis: 2000,
                    singleUndoCrossfade: true,
                    rippleUndo: true,
                    previewFollowsSeek: true,
                    stalePreviewRejectedAfterSave: true,
                    provenance: evidence
                };
                stage = 6;
                break;
            }
        case 6:
            {
                const header = itemNamed(assembly, "assemblyTrackHeader_0");
                require(header !== null, "track header missing");
                headerX = header.mapToItem(assembly, 0, 0).x;
                assembly.fitSelection();
                stage = 7;
                break;
            }
        case 7:
            {
                const header = itemNamed(assembly, "assemblyTrackHeader_0");
                require(assembly.scrollPosition > 0, "selection fit did not scroll");
                require(Math.abs(header.mapToItem(assembly, 0, 0).x - headerX) < 1, "track header scrolled out of position");
                facts.fixedHeaderAfterScroll = true;
                uiPrefs.languageMode = "en";
                uiPrefs.mode = 2;
                stage = 8;
                break;
            }
        case 8:
            require(itemNamed(assembly, "assemblyPreviewButton").text === "Preview", "live English translation failed");
            require(itemNamed(assembly, "assemblyFadeInCurve").currentIndex === 2, "English model reset the pinned fade curve");
            uiPrefs.languageMode = "zh_CN";
            uiPrefs.mode = 1;
            stage = 9;
            break;
        case 9:
            require(itemNamed(assembly, "assemblyPreviewButton").text === "试听", "live Chinese translation failed");
            require(itemNamed(assembly, "assemblyFadeInCurve").currentIndex === 2, "Chinese model reset the pinned fade curve");
            facts.liveLanguageSwitch = true;
            assembly.sourcesVisible = true;
            assembly.inspectorVisible = false;
            assembly.fitProject();
            assembly.memorySavedNotice = false;
            stage = 10;
            break;
        case 10:
            if(stress) {
                assembly.mutate(next=>{
                    const templates=next.tracks.map(track=>JSON.parse(JSON.stringify(track.clips[0])));
                    next.name="组合回归 · 8 tracks / 32 clips";
                    next.tracks=[];
                    for(let track=0;track<8;++track) {
                        const clips=[];
                        for(let index=0;index<4;++index) {
                            const clip=JSON.parse(JSON.stringify(templates[track%3]));
                            clip.id=backend.newAssemblyObjectId();clip.timelineStartMillis=index*7000;
                            clip.sourceStartMillis=100;clip.sourceEndMillis=8100;
                            clip.fadeInMillis=1000;clip.fadeOutMillis=1000;clip.gainCentibels=-300;
                            clips.push(clip);
                        }
                        next.tracks.push({id:backend.newAssemblyObjectId(),name:"Track "+(track+1),gainCentibels:-900,panPercent:track*20-70,muted:false,solo:false,clips:clips});
                    }
                });
                require(assembly.saveRevision()!==null,"stress revision failed to save");
                stressDocument=JSON.stringify(assembly.document.tracks);
                const project=assembly.document.id;
                shell.showAudioSpace();shell.showSoundAssembly();assembly.openAssembly(project);
                require(JSON.stringify(assembly.document.tracks)===stressDocument,"complex project lost pinned edits on reopen");
                assembly.fitProject();assembly.preview();
                const started=Date.now();soundAssemblyController.cancel();
                facts.cancelMillis=Date.now()-started;
                require(!soundAssemblyController.running,"cancel left active render");
                renderStarted=Date.now();heartbeatGaps=[];lastHeartbeat=0;
                assembly.preview();stage=11;break;
            }
            restorePreferences();
            reportJson = JSON.stringify({
                ok: true,
                facts: facts
            });
            break;
        case 11:
            if(soundAssemblyController.running) return;
            require(assembly.previewCurrent && player.playing,"complex preview not playable after cancellation");
            facts.complexPreviewMillis=Date.now()-renderStarted;
            assembly.stopPlayback();
            facts.complexTracks=assembly.tracks.length;facts.complexClips=32;
            const sorted=heartbeatGaps.slice().sort((a,b)=>a-b);
            facts.uiHeartbeatP95Millis=sorted.length?sorted[Math.floor((sorted.length-1)*.95)]:0;
            facts.uiHeartbeatMaxMillis=sorted.length?sorted[sorted.length-1]:0;
            assembly.keepMemory();stage=12;break;
        case 12: {
            if(soundAssemblyController.running) return;
            const memory=assets.find(asset=>asset.assemblyId===assembly.document.id && asset.inMemory);
            require(memory,"complex memory missing");
            const evidence=JSON.parse(memory.provenanceJson);
            require(evidence.sources.length===32,"complex memory lost clip provenance");
            facts.complexOutputPath=memory.path;facts.complexDurationMillis=assembly.durationMillis;
            facts.complexProvenance=evidence;
            stage=13;break;
        }
        case 13:
            restorePreferences();reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer {
        interval:16;repeat:true;running:smoke.stress && smoke.stage===11 && soundAssemblyController.running
        onTriggered:{const now=Date.now();if(smoke.lastHeartbeat && smoke.heartbeatGaps.length<10000) smoke.heartbeatGaps.push(now-smoke.lastHeartbeat);smoke.lastHeartbeat=now;}
    }
    Timer {
        interval: 500
        repeat: true
        running: smoke.fixtureRoot.length > 0 && !smoke.reportJson
        onTriggered: {
            try {
                if (++smoke.ticks > 240)
                    throw new Error("multitrack workflow timed out");
                smoke.step();
            } catch (error) {
                smoke.restorePreferences();
                smoke.reportJson = JSON.stringify({
                    ok: false,
                    stage: smoke.stage,
                    error: String(error)
                });
            }
        }
    }
}
