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
    property int batchIndex: 0
    property int singleIndex: 0
    readonly property var deliveryFormats: ["wav_pcm24","flac24","wav_pcm16","wav_float32","mp3","aac_m4a"]
    function deliveryProfile(format) { return {format:format,sampleRate:44100,channels:1,bitrateKbps:192,includeMemoryInfo:true}; }
    function deliveryExtension(format) { return format==="flac24"?"flac":format==="mp3"?"mp3":format==="aac_m4a"?"m4a":"wav"; }
    property var originalIds: []
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
            if(reopening) {
                require(editor.asset.hasGeneratedSource,"portable source label missing");
                editor.presentSourceDisclosure();require(editor.sourceDisclosureDialog.spans.length>0,"native source labels missing in reopened dialog");editor.sourceDisclosureDialog.close();
                editor.memoryInfoDialog.present(editor.asset.id,false);
                require(editor.memoryInfoDialog.notes === "个人回忆\nA quiet memory", "portable notes missing");
                require(editor.memoryInfoDialog.moments.length === 1, "portable moment missing");
                editor.memoryInfoDialog.close();facts.memoryInfoReopened=true;
                facts.reopenedDialogVerified=true;stage=5;return;
            }
            editor.memoryInfoDialog.present(editor.asset.id,false,100,200);
            editor.memoryInfoDialog.notes="个人回忆\nA quiet memory";
            editor.memoryInfoDialog.place="外婆家阳台";
            editor.memoryInfoDialog.timeDescription="大约 2020 年夏天";
            editor.memoryInfoDialog.addMoment();editor.memoryInfoDialog.moments[0].note="第一次叫爸爸";
            editor.memoryInfoDialog.save();require(!editor.memoryInfoDialog.errorText,editor.memoryInfoDialog.errorText);
            require(shell.projectDirty,"personal context did not dirty project");
            facts.memoryInfoSaved=true;
            editor.adjustment.setGain(-300);
            editor.adjustment.setChannelRepairEnabled(true);
            editor.adjustment.setChannelRepairParameter("invertLeft",true);
            editor.adjustment.addSpectralRepairRegion(100,500,500,1500);
            require(shell.flushDrafts(),"combined adjustment failed to save");
            before=JSON.stringify(editor.adjustment.snapshot());
            facts.combinedProcessing=["gain","channel polarity","spectral repair"];
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
            if(reopening) {reportJson=JSON.stringify({ok:true,facts:facts});return;}
            stage=30;break;
        case 30:
            editor.debugExport('file://'+fixtureRoot+'/single-'+deliveryFormats[singleIndex]+'.'+deliveryExtension(deliveryFormats[singleIndex]),deliveryProfile(deliveryFormats[singleIndex]));stage=31;break;
        case 31:
            if(renderExporter.running)return;
            require(renderExporter.hasResult,renderExporter.errorText || "single format delivery failed");
            if(++singleIndex<deliveryFormats.length){stage=30;return;}
            facts.singleFormats=deliveryFormats;
            stage=7;break;
        case 7:
            batchExporter.exportOptions=deliveryProfile(deliveryFormats[batchIndex]);
            batchExporter.start([editor.asset],'file://'+fixtureRoot+'/batch',deliveryFormats[batchIndex]);stage=8;break;
        case 8:
            if(batchExporter.running) return;
            require(batchExporter.hasResult && batchExporter.completedCount===1 && batchExporter.failedCount===0,batchExporter.errorText || "batch delivery failed");
            batchExporter.dismiss();
            if(++batchIndex<deliveryFormats.length) {stage=7;return;}
            facts.batchFormats=deliveryFormats;
            const mix=backend.createSoundAssembly("Disclosed mix",[editor.asset.id],"layered");
            require(!mix.error,mix.error || "mix creation failed");
            require(!backend.setMemoryInfo(mix.id,true,0,{notes:"Whole trip",place:"Several places",timeDescription:"A week",moments:[]}),"mix context failed");
            soundAssemblyController.exportOptions=deliveryProfile("aac_m4a");
            soundAssemblyController.exportAssembly(mix,'file://'+fixtureRoot+'/mix.m4a');stage=9;break;
        case 9:
            if(soundAssemblyController.running) return;
            require(soundAssemblyController.hasResult,soundAssemblyController.errorText || "mix export failed");
            facts.mixExport=soundAssemblyController.outputPath;
            renderedSpectralWorkingCopy.createFromSavedAsset(editor.asset);stage=10;break;
        case 10:
            if(renderedSpectralWorkingCopy.running) return;
            require(renderedSpectralWorkingCopy.hasResult,renderedSpectralWorkingCopy.errorText || "working copy failed");
            renderedSpectralWorkingCopy.eraseRegion(editor.asset.id,renderedSpectralWorkingCopy.workingCopyId,renderedSpectralWorkingCopy.cachePath,600,900,800,1600);stage=20;break;
        case 20:
            if(renderedSpectralWorkingCopy.running) return;
            require(renderedSpectralWorkingCopy.hasResult && renderedSpectralWorkingCopy.operationCount===1,renderedSpectralWorkingCopy.errorText || "working-copy erase failed");
            facts.workingCopyErased=true;
            renderExporter.exportOptions={format:"wav_pcm24",sampleRate:48000,channels:2,includeMemoryInfo:true};
            renderExporter.exportRenderedSpectralWorkingCopy(editor.asset.id,Number(editor.asset.adjustmentRevision||0),renderedSpectralWorkingCopy.workingCopyId,renderedSpectralWorkingCopy.cachePath,'file://'+fixtureRoot+'/working-copy.wav');stage=11;break;
        case 11:
            if(renderExporter.running) return;
            require(renderExporter.hasResult,renderExporter.errorText || "working-copy delivery failed");
            facts.workingCopyExport=renderExporter.outputPath;
            originalIds=backend.listAssets().map(a=>a.id);
            independentEditor.importAudio(['file://'+fixtureRoot+'/marked.wav','file://'+facts.mixExport,'file://'+fixtureRoot+'/working-copy.wav']);stage=12;break;
        case 12:
            if(independentEditor.busy) return;
            require(!independentEditor.errorText,independentEditor.errorText);
            const imported=backend.listAssets().filter(a=>originalIds.indexOf(a.id)<0);
            require(imported.length>0,"exported audio did not reimport");
            for(const source of imported) {
                require(source.hasGeneratedSource,"imported export lost source declaration");
                require(source.sourceDisclosure.sources[0].origin==="embedded_export","imported label origin missing");
                require(source.sourceDisclosure.sources[0].spans[0].endMillis===source.durationMillis,"imported label did not cover whole output");
            }
            editor.sourceDisclosureDialog.present(imported[0],0,0);
            require(editor.sourceDisclosureDialog.importedLabels && editor.sourceDisclosureDialog.spans.length>0,"native imported labels missing in dialog");
            editor.sourceDisclosureDialog.close();facts.importedDialogVerified=true;
            facts.importedCount=imported.length;
            facts.jobs=backend.jobStats();require(facts.jobs.pending===0 && facts.jobs.running===0 && facts.jobs.done===0 && facts.jobs.failed===0,"private round trip started library analysis");
            reportJson=JSON.stringify({ok:true,facts:facts});break;
        }
    }
    Timer { interval:200; repeat:true; running:smoke.fixtureRoot.length>0 && !smoke.reportJson; onTriggered: {
        try { if(++smoke.ticks>300) throw new Error("source disclosure timed out");smoke.step(); }
        catch(error) { smoke.reportJson=JSON.stringify({ok:false,stage:smoke.stage,error:String(error),facts:smoke.facts}); }
    } }
}
