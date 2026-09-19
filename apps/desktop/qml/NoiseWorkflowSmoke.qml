//! Packaged proof of captured evidence, playback, persistence and exported audio.
import QtQuick

Item {
    id: smoke
    required property var shell
    required property var editor
    required property var library
    required property string fixtureRoot
    property int stage: 0
    property int ticks: 0
    property int stageTicks: 0
    property string reportJson: ''
    property string oldLanguage: ''
    property int oldAppearance: -1
    property string captured: ''
    property bool capturePending: false
    property int captureStart: 100
    property var facts: ({})
    function require(value,message) { if(!value) throw new Error(message); }
    function next() { ++stage; stageTicks=0; }
    function restore() { player.stop(); if(oldLanguage) uiPrefs.languageMode=oldLanguage; if(oldAppearance>=0) uiPrefs.mode=oldAppearance; }
    function beginCapture(asset) {
        library.selectedFilter='all'; library.refreshAssets(); library.selectAssetOnly(asset); shell.showSoundEditor();
        editor.spectralFocus=true; editor.spectralEditor.noiseTools=true;
        editor.adjustment.clearSpectralRepairRegions(); editor.adjustment.setNoiseProfile(null);
        captureStart=asset.spectralRepair.noiseProfile && asset.spectralRepair.noiseProfile.captureStartMillis===100 ? 150 : 100;
        capturePending=true;
    }
    function step() {
        ++stageTicks;
        const draft=editor.adjustment;
        if(capturePending) {
            if(!editor.visible || spectrogramPreview.running || !spectrogramPreview.imageUrl) return;
            editor.captureNoise(captureStart,1100); capturePending=false; return;
        }
        switch(stage) {
        case 0: {
            const asset=backend.listAssets().find(value=>value.path.endsWith('Noise reduction — calibration.wav'));
            if(!asset) { if(ticks===1) backend.addRoot('file://'+fixtureRoot+'/sources'); return; }
            oldLanguage=uiPrefs.languageMode; oldAppearance=uiPrefs.mode; uiPrefs.languageMode='zh_CN'; uiPrefs.mode=1;
            beginCapture(asset); next(); break;
        }
        case 1:
            if(noiseProfile.running) return;
            facts.captureError=noiseProfile.error; facts.captureObsolete=editor.noiseCaptureObsolete;
            require(draft.spectralRepair.noiseProfile,'noise sample was not delivered');
            require(!draft.spectralRepair.noiseProfile.enabled,'capture unexpectedly enabled processing');
            facts.bins=draft.spectralRepair.noiseProfile.powerCentibels.length;
            require(facts.bins===1025,'incomplete captured evidence');
            editor.playFrom(200); next(); break;
        case 2:
            if(stageTicks<2) return;
            facts.originalNoisePeakDb=player.outputPeakDb;
            draft.editNoiseProfile('reductionCentibels',1800); draft.editNoiseProfile('enabled',true);
            editor.playFrom(200); next(); break;
        case 3:
            if(stageTicks<2) return;
            facts.reducedNoisePeakDb=player.outputPeakDb;
            require(facts.originalNoisePeakDb-facts.reducedNoisePeakDb>6,'preview did not reduce captured noise');
            player.stop();
            captured=JSON.stringify(draft.spectralRepair.noiseProfile);
            editor.captureNoise(200,1000); draft.setGain(100);
            next(); break;
        case 4:
            if(noiseProfile.running) return;
            require(editor.noiseCaptureObsolete,'stale capture was accepted into a changed draft');
            require(JSON.stringify(draft.spectralRepair.noiseProfile)===captured,'stale capture replaced current evidence');
            draft.undo(); editor.save(); require(!editor.dirty,'noise profile save failed');
            const saved=backend.listAssets().find(value=>value.id===editor.asset.id);
            require(JSON.stringify(draft.copySpectralRepair(saved.spectralRepair))===JSON.stringify(draft.spectralRepair),'saved projection lost noise evidence');
            facts.savedRevision=saved.adjustmentRevision;
            shell.showAudioSpace(); shell.showSoundEditor(); editor.debugAnalyzeOutput(); next(); break;
        case 5:
            if(loudnessAnalyzer.running) return;
            require(loudnessAnalyzer.hasResult,'noise analysis failed'); facts.analysisLufs=loudnessAnalyzer.integratedLufs;
            editor.debugExport('file://'+fixtureRoot+'/Noise reduction — result.wav'); next(); break;
        case 6:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,'calibration export failed'); facts.exportLufs=renderExporter.integratedLufs;
            require(Math.abs(facts.analysisLufs-facts.exportLufs)<0.1,'loudness analysis and export disagree');
            editor.auditionNoise(true); next(); break;
        case 7:
            if(stageTicks<3) return;
            require(player.playing && editor.diagnosticMode==='noise-residue','removed-sound audition failed');
            require(!draft.dirty && JSON.stringify(draft.spectralRepair.noiseProfile)===captured,'diagnostic residue changed saved data');
            player.stop(); facts.residueNotPersisted=true;
            uiPrefs.languageMode='en'; uiPrefs.mode=2; next(); break;
        case 8: {
            uiPrefs.languageMode='zh_CN'; uiPrefs.mode=1;
            const voice=backend.listAssets().find(value=>value.path.endsWith('旁白 — 底噪练习.wav'));
            require(voice,'narration fixture missing'); beginCapture(voice); next(); break;
        }
        case 9:
            if(noiseProfile.running) return;
            require(draft.spectralRepair.noiseProfile,'narration profile missing');
            draft.editNoiseProfile('enabled',true); editor.save(); require(!editor.dirty,'narration save failed');
            editor.debugExport('file://'+fixtureRoot+'/旁白 — 降噪后.wav'); next(); break;
        case 10:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,'narration export failed');
            facts.voiceExportPath=renderExporter.outputPath;
            editor.spectralEditor.lowHertz=50; editor.spectralEditor.highHertz=10000;
            editor.noiseCaptureObsolete=false; next(); break;
        case 11:
            if(stageTicks<3 || spectrogramPreview.running || !spectrogramPreview.imageUrl) return;
            restore(); reportJson=JSON.stringify({ok:true,facts:facts}); break;
        }
    }
    Connections { target: noiseProfile; function onProfileReady(identity,profile) {
        if(!smoke.fixtureRoot.length) return;
        smoke.facts.deliveredBins=profile.powerCentibels.length;
        smoke.facts.deliveredArray=Array.isArray(profile.powerCentibels);
    } }
    Timer { interval: 300; repeat: true; running: smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>220) throw new Error('noise workflow timed out'); smoke.step(); }
        catch(error) { smoke.restore(); smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
