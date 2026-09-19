//! Opt-in packaged integration contract; never runs without an explicit fixture.
//! Exercises the actual QML workspace handlers, native render and catalog APIs.
import QtQuick

Item {
    id: smoke
    required property var shell
    required property var assembly
    required property var editor
    required property var library
    required property var materials
    required property string materialPath
    property int stage: 0
    property string reportJson: ""
    property var original: null
    property var material: null
    property var memory: null
    property string clipId: ""
    property int pinnedRevision: 0
    property var facts: ({})

    function require(condition: bool, message: string): void {
        if (!condition) throw new Error(message);
    }
    function step(): void {
        const assets = backend.listAssets();
        switch (stage) {
        case 0:
            original = assets.find(asset => asset.inMemory && !asset.assemblyId);
            if (!original) return;
            shell.createSoundAssembly([original.id], "sequence");
            require(assembly.hasDocument, "initial project was not created");
            clipId = assembly.selectedClipId;
            const error = backend.importMaterial(materialPath, assembly.document.id, true, "ambience");
            require(!error, error);
            stage = 1;
            break;
        case 1:
            material = assets.find(asset => asset.inMaterials && asset.materialCategory === "ambience");
            if (!material || backend.projectMaterials(assembly.document.id).indexOf(material.id) < 0) return;
            require(!material.inMemory, "new material appeared in memories");
            require(material.path !== materialPath && material.path.indexOf("/media/materials/") >= 0, "material is not managed");
            require(backend.projectMaterials(assembly.document.id).indexOf(material.id) >= 0, "material absent from project bin");
            assembly.refreshAssemblies();
            assembly.addTrack();
            assembly.addLibraryAsset(material, "material");
            assembly.selectedClipId = clipId;
            assembly.selectedTrackIndex = 0;
            assembly.openClipEditor();
            stage = 2;
            break;
        case 2:
            require(editor.editingProjectClip && editor.asset.id === original.id, "precision route lost the selected source");
            editor.debugNudgeEqualizer();
            require(editor.dirty, "precision edit did not change the draft");
            editor.save();
            require(!editor.dirty, "project clip save failed");
            pinnedRevision = assembly.selectedClip.adjustmentRevisionId;
            require(pinnedRevision > 0 && pinnedRevision !== Number(original.adjustmentRevision), "clip did not pin its new version");
            const unchanged = backend.listAssets().find(asset => asset.id === original.id);
            require(unchanged.adjustmentRevision === original.adjustmentRevision, "project edit changed library recording");
            editor.returnToProjectRequested();
            require(shell.workspaceIndex === 3 && assembly.selectedClipId === clipId, "return lost project selection");
            stage = 3;
            break;
        case 3:
            assembly.keepMemory();
            stage = 4;
            break;
        case 4:
            require(!soundAssemblyController.errorText, soundAssemblyController.errorText);
            memory = assets.find(asset => asset.assemblyId === assembly.document.id && asset.inMemory);
            if (!memory) return;
            const provenance = JSON.parse(memory.provenanceJson);
            require(provenance.sources.length === 2, "mix provenance lost a source");
            require(provenance.sources.some(source => source.sourceRole === "material" && source.assetId === material.id), "material role missing from provenance");
            require(provenance.sources.some(source => source.clipId === clipId && source.adjustmentRevisionId === pinnedRevision), "edited source revision missing from provenance");
            require(backend.waveformForAsset(memory.id).length > 0, "saved mix has no waveform");
            assembly.setMasterValue("gainCentibels", -100);
            require(assembly.saveRevision() !== null, "later draft could not be saved");
            const retained = backend.listAssets().find(asset => asset.id === memory.id);
            require(retained.path === memory.path && retained.assemblyRevisionId === memory.assemblyRevisionId, "draft replaced accepted listening edition");
            shell.showAudioSpace();
            library.selectedFilter = "all";
            library.refreshAssets();
            library.selectAssetOnly(retained);
            facts = { projectId: memory.id, originalId: original.id, materialId: material.id,
                materialPath: material.path, outputPath: memory.path, clipRevision: pinnedRevision,
                acceptedRevision: memory.assemblyRevisionId, draftRevision: assembly.document.revisionId,
                provenance: provenance, waveform: true, originalUnchanged: true, draftPreservedEdition: true };
            stage = 5;
            break;
        case 5:
            shell.showSoundEditor();
            require(shell.workspaceIndex === 3 && assembly.document.id === memory.id, "saved memory did not reopen its project");
            assembly.selectedClipId = clipId;
            assembly.openClipEditor();
            require(editor.asset.adjustmentRevision === pinnedRevision, "reopened clip lost isolated version");
            stage = 6;
            break;
        case 6:
            editor.returnToProjectRequested();
            shell.workspaceIndex = 4;
            materials.refresh();
            materials.category = "ambience";
            require(materials.filteredAssets.some(asset => asset.id === material.id), "global material category lost the imported source");
            require(materials.filteredAssets.every(asset => asset.inMaterials), "material browser contains an uncollected source");
            stage = 7;
            break;
        case 7:
            reportJson = JSON.stringify({ok: true, facts: facts});
            break;
        }
    }
    Timer {
        interval: 500
        repeat: true
        running: smoke.materialPath.length > 0 && !smoke.reportJson
        onTriggered: {
            try { smoke.step(); }
            catch (error) { smoke.reportJson = JSON.stringify({ok: false, stage: smoke.stage, error: String(error)}); }
        }
    }
}
