//! Opt-in packaged proof: spectral audition, save, reopen, analysis and export.
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
    property string originalLanguage: ''
    property int originalAppearance: -1
    property string assetId: ''
    property int calibrationEnd: 8000
    property int voiceStart: 3900
    property var facts: ({})
    function require(value,message) {if(!value) throw new Error(message);}
    function advance() {++stage; stageTicks=0;}
    function restore() {player.stop();if(originalLanguage) uiPrefs.languageMode=originalLanguage;if(originalAppearance>=0) uiPrefs.mode=originalAppearance;}
    function step() {
        ++stageTicks;
        const view=editor.spectralEditor, draft=editor.adjustment;
        switch(stage) {
        case 0: {
            const asset=backend.listAssets().find(value=>value.path.endsWith('Spectral repair — calibration.wav'));
            if(!asset) {if(ticks===1) backend.addRoot("file://"+fixtureRoot+"/sources");return;}
            originalLanguage=uiPrefs.languageMode;originalAppearance=uiPrefs.mode;
            uiPrefs.languageMode='zh_CN';uiPrefs.mode=1;
            calibrationEnd=asset.spectralRepair.regions.length && asset.spectralRepair.regions[0].endMillis===8000 ? 7900 : 8000;
            assetId=asset.id; library.selectedFilter="all"; library.refreshAssets(); library.selectAssetOnly(asset); shell.showSoundEditor();
            editor.spectralFocus=true;
            advance();break;
        }
        case 1:
            if(spectrogramPreview.running || !spectrogramPreview.imageUrl) return;
            draft.clearSpectralRepairRegions();draft.setLimiterEnabled(false);
            editor.auditionOriginal=true;editor.playFrom(1000);
            advance();break;
        case 2:
            if(stageTicks<4) return;
            require(player.playing,'original playback not active');
            facts.originalPeakDb=player.outputPeakDb;
            player.stop();editor.auditionOriginal=false;
            view.setSelection({startMillis:0,endMillis:calibrationEnd,lowHertz:900,highHertz:1100,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:40},-1);
            view.applySelection();require(draft.spectralRepair.regions.length===1,'repair was not authored');
            editor.playFrom(1000);advance();break;
        case 3:
            if(stageTicks<4) return;
            facts.repairedPeakDb=player.outputPeakDb;
            require(facts.originalPeakDb-facts.repairedPeakDb>8,'editor preview omitted spectral repair');
            view.editField('attenuationCentibels',1200);
            require(!player.active,'old repaired preview still active after edit');
            draft.undo();require(draft.spectralRepair.regions[0].attenuationCentibels===2400,'undo failed');
            editor.save();require(!editor.dirty,'repair save failed');
            facts.revisionId=backend.listAssets().find(value=>value.id===assetId).adjustmentRevision;
            facts.afterSave=JSON.stringify(draft.spectralRepair);
            facts.afterSaveAsset=JSON.stringify(editor.asset.spectralRepair);
            facts.afterSaveBackend=JSON.stringify(draft.copySpectralRepair(backend.listAssets().find(value=>value.id===assetId).spectralRepair));
            require(facts.afterSave===facts.afterSaveBackend,'catalog projection lost saved spectral repair');
            shell.showAudioSpace();shell.showSoundEditor();
            editor.debugAnalyzeOutput();advance();break;
        case 4:
            if(loudnessAnalyzer.running) return;
            require(loudnessAnalyzer.hasResult,'analysis did not complete: '+loudnessAnalyzer.errorText);
            facts.analysisLufs=loudnessAnalyzer.integratedLufs;
            require(draft.spectralRepair.regions.length===1,'reopened repair lost its region: '+editor.asset.id+' expected '+assetId);
            editor.debugExport('file://'+fixtureRoot+'/Spectral repair — result.wav');advance();break;
        case 5:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,'export failed: '+renderExporter.errorText);
            facts.exportPath=renderExporter.outputPath;facts.exportLufs=renderExporter.integratedLufs;
            require(Math.abs(facts.analysisLufs-facts.exportLufs)<0.1,'analysis and export disagree');
            view.setSelection(draft.spectralRepair.regions[0],0);
            editor.auditionSpectralSelection(view.selection,true);
            advance();break;
        case 6:
            if(stageTicks<3) return;
            require(editor.listeningToSourceBand && player.playing,'source-band audition failed');
            facts.sourceBandPeakDb=player.outputPeakDb;player.stop();
            facts.sourceBandNotPersisted=draft.spectralRepair.regions.length===1&&!draft.dirty;
            require(facts.sourceBandNotPersisted,'diagnostic filter leaked into the document');
            view.lowHertz=20;view.highHertz=1000;
            view.lowHertz=300;view.highHertz=6000;
            advance();break;
        case 7:
            if(spectrogramPreview.running || !spectrogramPreview.imageUrl) return;
            uiPrefs.languageMode='en';uiPrefs.mode=2;advance();break;
        case 8:
            require(draft.spectralRepair.regions.length===1,'locale changed repair data');
            uiPrefs.languageMode='zh_CN';uiPrefs.mode=1;advance();break;
        case 9:
            if(stageTicks<2) return;
            facts.localeAndAppearanceSwitch=true;facts.latestViewport=true;
            {
                const voice=backend.listAssets().find(value=>value.path.endsWith('旁白 — 啸叫练习.wav'));
                require(voice,'real narration fixture missing');
                voiceStart=voice.spectralRepair.regions.length && voice.spectralRepair.regions[0].startMillis===3900 ? 3850 : 3900;
                library.selectAssetOnly(voice);shell.showSoundEditor();advance();break;
            }
        case 10:
            if(spectrogramPreview.running || !spectrogramPreview.imageUrl) return;
            draft.clearSpectralRepairRegions();
            view.setSelection({startMillis:voiceStart,endMillis:7100,lowHertz:1650,highHertz:1950,attenuationCentibels:2400,timeFeatherMillis:30,frequencyFeatherHertz:60},-1);
            view.applySelection();
            facts.voiceBeforeSave=JSON.stringify(draft.spectralRepair);
            editor.save();require(!editor.dirty,'voice repair save failed');
            facts.voiceAfterSave=JSON.stringify(draft.spectralRepair);
            facts.voiceBackend=JSON.stringify(draft.copySpectralRepair(backend.listAssets().find(value=>value.id===editor.asset.id).spectralRepair));
            facts.voiceAsset=JSON.stringify(editor.asset.spectralRepair);
            require(facts.voiceAfterSave===facts.voiceBackend,'voice catalog projection lost repair');
            require(draft.spectralRepair.regions.length===1,'voice repair lost after save');
            editor.debugExport('file://'+fixtureRoot+'/旁白 — 修复后.wav');advance();break;
        case 11:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,'voice repair export failed');
            facts.voiceExportPath=renderExporter.outputPath;
            view.setSelection(draft.spectralRepair.regions[0],0);
            view.lowHertz=200;view.highHertz=8000;
            advance();break;
        case 12:
            if(spectrogramPreview.running || !spectrogramPreview.imageUrl) return;
            if(stageTicks<3) return;
            restore();reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer { interval: 450; repeat: true; running: smoke.fixtureRoot.length>0&&!smoke.reportJson; onTriggered: {
        try {if(++smoke.ticks>150) throw new Error('spectral workflow timed out');smoke.step();}
        catch(error) {smoke.restore();smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts});}
    } }
}
