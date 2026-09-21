import QtQuick
import QtQuick.Dialogs
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "TranscriptDelivery"
    width: 800; height: 500
    visible: true; when: windowShown
    QtObject {
        id: transcriptExporter
        property var saved: null
        property string error: ""
        function hasTiming(record) { return record.segments.length > 0; }
        function copyText(record) { return ""; }
        function save(record,url,format,source) { saved={record:record,url:url,format:format,source:source};return error; }
    }
    TranscriptExportMenu { id: delivery; record: null; sourcePath: "" }
    function init() {
        delivery.record={text:"Original transcript",segments:[{start:1,end:2,text:"Original transcript"}]};
        delivery.sourcePath="/original.wav"; transcriptExporter.saved=null; transcriptExporter.error="";
        findChild(delivery,"transcriptExportFile").options=FileDialog.DontUseNativeDialog;
    }
    function cleanup() {
        findChild(delivery,"transcriptExportFormats").close();
        findChild(delivery,"transcriptExportFile").close();
    }
    function test_menu_and_save_dialog_pin_the_requested_record() {
        mouseClick(findChild(delivery,"exportTranscript"));
        tryCompare(findChild(delivery,"transcriptExportFormats"),"visible",true);
        mouseClick(findChild(delivery,"exportTranscriptText"));
        const dialog=findChild(delivery,"transcriptExportFile");
        tryCompare(dialog,"visible",true);
        compare(dialog.defaultSuffix,"txt");
        delivery.record={text:"Another source",segments:[]};delivery.sourcePath="/other.wav";
        dialog.selectedFile="file:///tmp/echo-transcript-contract.txt";
        dialog.accepted();
        compare(transcriptExporter.saved.record.text,"Original transcript");
        compare(transcriptExporter.saved.source,"/original.wav");
        compare(transcriptExporter.saved.format,"txt");
        verify(delivery.notice.length>0);
    }
    function test_cancel_does_not_export_and_errors_remain_visible() {
        delivery.prepareExport("vtt");
        const dialog=findChild(delivery,"transcriptExportFile");
        tryCompare(dialog,"visible",true);dialog.rejected();dialog.close();
        compare(transcriptExporter.saved,null);compare(delivery.pendingSource,"");
        delivery.prepareExport("srt");transcriptExporter.error="Destination unavailable";
        dialog.selectedFile="file:///tmp/echo-transcript-contract.srt";dialog.accepted();
        compare(delivery.notice,"Destination unavailable");
        compare(transcriptExporter.saved.format,"srt");
    }
}
