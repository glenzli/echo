import QtQuick
import QtTest
import EchoDesktop

TestCase {
    id: testCase
    name: 'SpectralRepairGestures'
    visible: true; width: 1280; height: 680; when: windowShown
    property list<int> nativeNoisePowers: []
    SoundAdjustmentDraft { id: draft; asset: null }
    SignalSpy { id: captureSpy; target: view; signalName: 'noiseCaptureRequested' }
    SpectrogramView {
        id: view; anchors.fill: parent
        imageUrl: ''; loading: false; sourceDurationMillis: 8000
        viewStartRatio: 0; viewEndRatio: 1; progress: 0
        hasTimeSelection: false; selectionStartRatio: 0; selectionEndRatio: 0
        layerEnabled: draft.spectralRepair.enabled; regions: draft.spectralRepair.regions
        noiseSettings: draft.spectralRepair.noiseProfile || null
        onNoiseEdited: (key,value) => draft.editNoiseProfile(key,value)
        onNoiseClearRequested: draft.setNoiseProfile(null)
        canCreateRenderedWorkingCopy: false; hasRenderedWorkingCopy: false; renderedWorkingCopyRunning: false
        renderedWorkingCopyReady: false; renderedWorkingCopyError: ''; renderedEraseMode: false; renderedWorkingCopyOperationCount: 0
        onRegionsAppended: values => draft.appendSpectralRepairRegions(values)
        onRegionUpdated: (index,value) => draft.updateSpectralRepairRegion(index,value)
        onRegionRemoved: index => draft.removeSpectralRepairRegion(index)
        onGestureStarted: draft.beginGesture(); onGestureFinished: draft.endGesture()
    }
    function init() {
        draft.asset=null;
        const asset=draft.assetSnapshot(); asset.durationMillis=8000; asset.trimEndMillis=8000; asset.id="fixture"; draft.asset=asset;
        draft.clearSpectralRepairRegions(); view.resetSelection(); view.noiseTools=false; captureSpy.clear(); wait(20);
    }
    function drag(surface,x,y,dx,dy,modifiers) {
        mousePress(surface,x,y,Qt.LeftButton,modifiers||Qt.NoModifier);
        mouseMove(surface,x+dx/2,y+dy/2,20,Qt.LeftButton,modifiers||Qt.NoModifier);
        mouseMove(surface,x+dx,y+dy,20,Qt.LeftButton,modifiers||Qt.NoModifier);
        mouseRelease(surface,x+dx,y+dy,Qt.LeftButton,modifiers||Qt.NoModifier);
    }
    function test_background_metadata_preserves_authored_repairs() {
        draft.addSpectralRepairRegion(1000,2000,900,1100);
        const before=JSON.stringify(draft.spectralRepair);
        draft.asset=Object.assign({},draft.asset,{soundCaption:'New model label',analysisState:'complete'});
        compare(JSON.stringify(draft.spectralRepair),before);
        verify(draft.canUndo);draft.undo();compare(draft.spectralRepair.regions.length,0);
    }
    function test_saved_revision_reloads_all_region_fields() {
        draft.addSpectralRepairRegion(1000,2000,900,1100);
        const expected=JSON.stringify(draft.spectralRepair);
        draft.asset=Object.assign({},draft.asset,{adjustmentRevision:1,spectralRepair:JSON.parse(expected)});
        compare(JSON.stringify(draft.spectralRepair),expected);
        verify(!draft.dirty);
    }
    function test_selection_is_non_destructive_until_applied() {
        const surface=findChild(view,'spectralSurface');
        drag(surface,200,60,180,120);
        verify(view.selection!==null); compare(draft.spectralRepair.regions.length,0);
        mouseClick(findChild(view,'applySpectralSelection'));
        compare(draft.spectralRepair.regions.length,1);
        draft.undo(); compare(draft.spectralRepair.regions.length,0);
        draft.redo(); compare(draft.spectralRepair.regions.length,1);
    }
    function test_move_resize_and_single_undo() {
        view.setSelection({startMillis:1000,endMillis:3000,lowHertz:500,highHertz:3000,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:40},-1);
        view.applySelection(); wait(20);
        const surface=findChild(view,'spectralSurface'), before=JSON.stringify(draft.spectralRepair.regions);
        drag(surface,surface.xAt(2000),surface.yAt(1500),70,0);
        verify(draft.spectralRepair.regions[0].startMillis>1000);
        draft.undo(); compare(JSON.stringify(draft.spectralRepair.regions),before);
        view.setSelection(draft.spectralRepair.regions[0],0);
        drag(surface,surface.xAt(3000)-1,surface.yAt(1500),60,0);
        verify(draft.spectralRepair.regions[0].endMillis>3000);
    }
    function test_numeric_control_updates_hertz() {
        view.setSelection({startMillis:1000,endMillis:3000,lowHertz:950,highHertz:1050,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:20},-1);
        view.applySelection();wait(20);
        const field=findChild(view,'spectralLow');
        mouseClick(field.up.indicator);
        wait(30);
        compare(draft.spectralRepair.regions[0].lowHertz,951);
    }
    function test_numeric_change_and_harmonics_are_reversible() {
        view.setSelection({startMillis:1000,endMillis:3000,lowHertz:950,highHertz:1050,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:20},-1);
        view.applySelection(); view.editField('attenuationCentibels',1200);
        compare(draft.spectralRepair.regions[0].attenuationCentibels,1200);
        draft.undo(); compare(draft.spectralRepair.regions[0].attenuationCentibels,2400);
        view.addHarmonics(4); compare(draft.spectralRepair.regions.length,4);
        draft.undo(); compare(draft.spectralRepair.regions.length,1);
        view.addHarmonics(4); view.addHarmonics(4); compare(draft.spectralRepair.regions.length,4);
    }
    function test_harmonics_apply_new_selection_once() {
        const selection={startMillis:1000,endMillis:3000,lowHertz:950,highHertz:1050,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:20};
        view.setSelection(selection,-1);
        view.addHarmonics(4); compare(draft.spectralRepair.regions.length,4); compare(view.selectedIndex,0);
        view.setSelection(selection,-1);
        view.addHarmonics(4); compare(draft.spectralRepair.regions.length,4); compare(view.selectedIndex,0);
        view.editField('attenuationCentibels',1200);
        compare(draft.spectralRepair.regions[0].attenuationCentibels,1200);
        draft.undo(); compare(draft.spectralRepair.regions[0].attenuationCentibels,2400);
        draft.undo(); compare(draft.spectralRepair.regions.length,0);
    }
    function test_noise_profile_enable_clear_and_undo_preserve_evidence() {
        const profile={algorithmVersion:1,enabled:false,captureStartMillis:100,captureEndMillis:900,
            powerCentibels:Array(1025).fill(-6000),reductionCentibels:1200,sensitivityCentibels:600,smoothingBins:3};
        draft.setNoiseProfile(profile); view.noiseTools=true; wait(20);
        verify(!draft.spectralRepair.noiseProfile.enabled);
        mouseClick(findChild(view,'enableNoiseProfile')); wait(20);
        verify(draft.spectralRepair.noiseProfile.enabled);
        draft.undo(); verify(!draft.spectralRepair.noiseProfile.enabled);
        draft.redo(); verify(draft.spectralRepair.noiseProfile.enabled);
        draft.addSpectralRepairRegion(1000,2000,900,1100); draft.clearSpectralRepairRegions();
        compare(draft.spectralRepair.noiseProfile.powerCentibels.length,1025);
        draft.setNoiseProfile(null); verify(!draft.spectralRepair.noiseProfile);
        draft.undo(); compare(draft.spectralRepair.noiseProfile.powerCentibels.length,1025);
        const saved=JSON.stringify(draft.spectralRepair);
        draft.setNoiseProfile(Object.assign({},profile,{powerCentibels:[0]}));
        compare(JSON.stringify(draft.spectralRepair),saved);
        draft.asset=Object.assign({},draft.asset,{adjustmentRevision:7,spectralRepair:JSON.parse(saved)});
        compare(JSON.stringify(draft.spectralRepair),saved); verify(!draft.dirty);
    }
    function test_noise_capture_button_uses_a_bounded_time_selection() {
        view.noiseTools=true;
        view.setSelection({startMillis:100,endMillis:1100,lowHertz:100,highHertz:1000,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:20},-1);
        // Native layout/render timing need not fit inside one 20 ms frame.
        verify(waitForPolish(view.Window.window));
        verify(waitForRendering(view));
        mouseClick(findChild(view,'captureNoise'));
        compare(captureSpy.count,1);
        compare(captureSpy.signalArguments[0][0],100); compare(captureSpy.signalArguments[0][1],1100);
        view.setSelection(Object.assign({},view.selection,{endMillis:150}),-1);
        verify(!findChild(view,'captureNoise').enabled);
        compare(draft.spectralRepair.regions.length,0);
    }
    function test_native_noise_sequence_is_copied_to_stable_history() {
        nativeNoisePowers=Array(1025).fill(-6100);
        draft.setNoiseProfile({algorithmVersion:1,enabled:false,captureStartMillis:100,captureEndMillis:900,
            powerCentibels:nativeNoisePowers,reductionCentibels:1200,sensitivityCentibels:600,smoothingBins:3});
        verify(draft.spectralRepair.noiseProfile!==undefined);
        compare(draft.spectralRepair.noiseProfile.powerCentibels.length,1025);
        nativeNoisePowers[0]=-3000;
        compare(draft.spectralRepair.noiseProfile.powerCentibels[0],-6100);
        draft.undo(); verify(!draft.spectralRepair.noiseProfile);
        draft.redo(); compare(draft.spectralRepair.noiseProfile.powerCentibels[0],-6100);
    }
}
