import QtQuick
import QtTest
import EchoDesktop

TestCase {
    id: testCase
    name: 'SpectralRepairGestures'
    visible: true; width: 1280; height: 680; when: windowShown
    SoundAdjustmentDraft { id: draft; asset: null }
    SpectrogramView {
        id: view; anchors.fill: parent
        imageUrl: ''; loading: false; sourceDurationMillis: 8000
        viewStartRatio: 0; viewEndRatio: 1; progress: 0
        hasTimeSelection: false; selectionStartRatio: 0; selectionEndRatio: 0
        layerEnabled: draft.spectralRepair.enabled; regions: draft.spectralRepair.regions
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
        draft.clearSpectralRepairRegions(); view.resetSelection(); wait(20);
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
}
