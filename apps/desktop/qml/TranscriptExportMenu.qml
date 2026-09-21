//! Copy/export a pinned transcript record without changing the audio or its evidence.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

RowLayout {
    id: control
    required property var record
    required property string sourcePath
    property var selectionRecord: null
    readonly property bool hasSelection: selectionRecord !== null && selectionRecord.segments.length > 0
    readonly property bool selectionHasTiming: hasSelection && transcriptExporter.hasTiming(selectionRecord)
    property var pendingRecord: ({})
    property string pendingSource: ""
    property string format: "txt"
    property string notice: ""
    readonly property bool hasText: record !== null && String(record.text || "").trim().length > 0
    readonly property bool hasTiming: record !== null && transcriptExporter.hasTiming(record)
    onRecordChanged: notice = ""
    spacing: 4
    function prepareExport(kind, selected): void {
        format = kind;
        pendingRecord = JSON.parse(JSON.stringify(selected ? selectionRecord : record));
        pendingSource = sourcePath;
        notice = "";
        fileDialog.open();
    }
    function saveTo(url: url): void {
        const error = transcriptExporter.save(pendingRecord, url, format, pendingSource);
        notice = error || qsTr("Transcript exported.");
        pendingRecord = ({}); pendingSource = "";
    }
    EchoButton {
        objectName: "copyTranscript"
        text: control.hasSelection ? qsTr("Copy selected") : qsTr("Copy text"); ghost: true; enabled: control.hasSelection || control.hasText
        onClicked: control.notice = transcriptExporter.copyText(control.hasSelection ? control.selectionRecord : control.record) || qsTr("Transcript copied.")
    }
    EchoIconButton {
        objectName: "exportTranscript"
        source: "qrc:/EchoDesktop/icons/export.svg"
        toolTipText: qsTr("Export transcript"); enabled: control.hasText || control.hasTiming
        onClicked: exportMenu.popup()
        Menu {
            id: exportMenu
            objectName: "transcriptExportFormats"
            MenuItem { text: qsTr("Copy complete transcript"); visible: control.hasSelection; enabled: control.hasText; onTriggered: control.notice = transcriptExporter.copyText(control.record) || qsTr("Transcript copied.") }
            MenuSeparator { visible: control.hasSelection }
            MenuItem { objectName: "exportTranscriptText"; text: qsTr("Plain text (.txt)"); enabled: control.hasText; onTriggered: control.prepareExport("txt") }
            MenuItem { text: qsTr("SubRip subtitles (.srt)"); enabled: control.hasTiming; onTriggered: control.prepareExport("srt") }
            MenuItem { text: qsTr("WebVTT subtitles (.vtt)"); enabled: control.hasTiming; onTriggered: control.prepareExport("vtt") }
            MenuSeparator { visible: control.hasSelection }
            MenuItem { objectName: "exportSelectedTranscriptText"; text: qsTr("Selected text (.txt)"); visible: control.hasSelection; onTriggered: control.prepareExport("txt",true) }
            MenuItem { objectName: "exportSelectedTranscriptSrt"; text: qsTr("Selected subtitles (.srt)"); visible: control.hasSelection; enabled: control.selectionHasTiming; onTriggered: control.prepareExport("srt",true) }
            MenuItem { text: qsTr("Selected subtitles (.vtt)"); visible: control.hasSelection; enabled: control.selectionHasTiming; onTriggered: control.prepareExport("vtt",true) }
        }
    }
    FileDialog {
        id: fileDialog
        objectName: "transcriptExportFile"
        title: qsTr("Export transcript"); fileMode: FileDialog.SaveFile
        nameFilters: control.format === "srt" ? [qsTr("SubRip subtitles (*.srt)")] : control.format === "vtt" ? [qsTr("WebVTT subtitles (*.vtt)")] : [qsTr("Plain text (*.txt)")]
        defaultSuffix: control.format
        onAccepted: control.saveTo(selectedFile)
        onRejected: { control.pendingRecord = ({}); control.pendingSource = ""; }
    }
}
