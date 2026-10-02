import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test; name: "SoundExport"; width: 900; height: 900; visible: true; when: windowShown
    property int saves: 0
    property bool saveFails: false
    property var source: ({id:"synthetic",path:"/synthetic.wav",pathStatus:"available",adjustmentRevision:3})
    QtObject {
        id: draft
        property bool dirty: true
        property int trimStartMillis: 500; property int trimEndMillis: 3500
        property int fadeInMillis: 100; property int fadeOutMillis: 200
        property int fadeInCurve: 0; property int fadeOutCurve: 0; property int gainCentibels: -600
        property int lowCutHertz: 0; property bool equalizerEnabled: false; property var equalizerBands: []
        property bool compressorEnabled: false; property int compressorThresholdCentibels: -1800
        property int compressorRatioTenths: 30; property int compressorAttackMillis: 10
        property int compressorReleaseMillis: 120; property int compressorMakeupCentibels: 0
        property bool limiterEnabled: false; property int limiterCeilingCentibels: -100; property int limiterReleaseMillis: 100
        property var effectChain: []; property var editSegments: []; property var effectMasks: []
        function restorationValue() {return {};}
        function deHumValue() {return {};}
        function deClickValue() {return {};}
        function channelRepairValue() {return {};}
        function reverbValue() {return {};}
        function creativeVfxValue() {return {};}
        function spectralRepairValue() {return {};}
    }
    QtObject {
        id: exporter
        property bool running: false; property bool hasResult: false; property string errorText: ""
        property real progress: 0; property real integratedLufs: -18; property real truePeakDbtp: -2
        property string outputPath: ""; property var exportOptions: ({})
        property int calls: 0; property var last: []
        function exportAdjusted() {++calls;last=Array.prototype.slice.call(arguments);running=true;}
        function exportRenderedSpectralWorkingCopy() {++calls;last=Array.prototype.slice.call(arguments);running=true;}
        function cancel() {running=false;}
    }
    SoundExportDialog {id: dialog;asset:test.source;draft:draft;exporter:exporter;renderedWorkingCopy:null}
    Connections {
        target: dialog; ignoreUnknownSignals: true
        function onSaveRequested() {
            ++test.saves;
            if (!test.saveFails) {test.source={id:"synthetic",path:"/synthetic.wav",pathStatus:"available",adjustmentRevision:4};draft.dirty=false;}
        }
    }
    function button(root) {
        if (root.text === "Export audio" || root.text === "Save and export") return root;
        for (const child of root.children || []) {const found=button(child);if(found)return found;}
        return null;
    }
    function init() {
        exporter.running=false;dialog.close();tryCompare(dialog,"visible",false);
        saves=0;saveFails=false;draft.dirty=true;exporter.calls=0;exporter.last=[];exporter.hasResult=false;exporter.errorText="";exporter.outputPath="";
        source={id:"synthetic",path:"/synthetic.wav",pathStatus:"available",adjustmentRevision:3};
        dialog.renderedWorkingCopy=null;dialog.present();tryCompare(dialog,"opened",true);
        dialog.destination="file:///synthetic-delivery.wav";
    }
    function cleanup() {exporter.running=false;dialog.close();}
    function test_dirty_draft_saves_and_exports_from_one_explicit_action() {
        const start=button(dialog.contentItem);verify(start!==null);verify(start.enabled,"dirty draft needs a direct save-and-export action");
        mouseClick(start);compare(saves,1);compare(exporter.calls,1);compare(exporter.last[1],4);
        compare(exporter.last[4],500);compare(exporter.last[5],3500);compare(exporter.last[6],100);compare(exporter.last[7],200);compare(exporter.last[10],-600);
    }
    function test_failed_save_keeps_destination_and_draft_for_retry() {
        saveFails=true;dialog.startExport();compare(exporter.calls,0);compare(saves,1);verify(draft.dirty);
        compare(dialog.destination.toString(),"file:///synthetic-delivery.wav");
        saveFails=false;dialog.startExport();compare(exporter.calls,1);compare(saves,2);
    }
    function test_file_created_with_record_error_is_distinct_from_render_failure() {
        draft.dirty=false;dialog.startExport();
        exporter.outputPath="/synthetic-delivery.wav";exporter.errorText="Source labels changed";exporter.running=false;
        const receipt=findChild(dialog,"exportReceipt"), failure=findChild(dialog,"exportFailure"), path=findChild(dialog,"exportCreatedPath");
        verify(receipt && receipt.visible);compare(receipt.text,"Audio created, but Echo could not save its source record.");
        verify(!failure.visible);verify(path.visible);verify(path.text.includes("/synthetic-delivery.wav"));
        dialog.close();dialog.present();tryCompare(dialog,"opened",true);
        verify(!receipt.visible && !failure.visible && !path.visible,"new export dialog must not show an old receipt");
    }
    function test_failed_new_save_does_not_reuse_the_previous_delivery_receipt() {
        draft.dirty=false;dialog.startExport();
        exporter.outputPath="/synthetic-delivery.wav";exporter.hasResult=true;exporter.running=false;
        verify(findChild(dialog,"exportReceipt").visible);
        draft.dirty=true;saveFails=true;dialog.startExport();
        compare(exporter.calls,1);verify(draft.dirty);verify(dialog.saveErrorText.length>0);
        verify(!findChild(dialog,"exportReceipt").visible && !findChild(dialog,"exportCreatedPath").visible);
    }
    function test_complete_failure_and_cancel_do_not_claim_a_created_file() {
        draft.dirty=false;dialog.startExport();exporter.running=false;exporter.errorText="Destination is unavailable";
        verify(findChild(dialog,"exportFailure").visible);verify(!findChild(dialog,"exportReceipt").visible);
        exporter.errorText="";
        verify(!findChild(dialog,"exportFailure").visible && !findChild(dialog,"exportCreatedPath").visible);
    }
    function test_incomplete_destination_and_missing_source_do_not_save_or_render() {
        dialog.destination="";dialog.startExport();compare(saves,0);compare(exporter.calls,0);
        dialog.destination="file:///synthetic-delivery.wav";
        source={id:"synthetic",path:"/synthetic.wav",pathStatus:"missing",adjustmentRevision:3};
        dialog.startExport();compare(saves,0);compare(exporter.calls,0);
    }
    function test_clean_or_frozen_copy_export_does_not_publish_an_unrelated_draft() {
        draft.dirty=false;dialog.startExport();compare(saves,0);compare(exporter.calls,1);
        exporter.running=false;draft.dirty=true;
        dialog.renderedWorkingCopy={id:7,cachePath:"/frozen.wav",operationCount:1};dialog.exportRenderedWorkingCopy=true;
        dialog.startExport();compare(saves,0);verify(draft.dirty);compare(exporter.calls,2);compare(exporter.last[2],7);
    }
}
