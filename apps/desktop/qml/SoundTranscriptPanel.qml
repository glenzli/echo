//! Sentence-level AI evidence to original-time selections. Audio edits stay in the draft.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SoundTranscriptEditing.js" as TranscriptEditing

Rectangle {
    id: panel
    required property var asset
    required property string revisionKey
    required property real rangeStart
    required property real rangeEnd
    required property real trimStart
    required property real trimEnd
    property var records: []
    property int recordIndex: 0
    property bool unitMode: false
    property string notice: ""
    property string acceptedKey: ""
    readonly property string currentKey: asset ? asset.id + ":" + revisionKey + ":" + rangeStart + ":" + rangeEnd : ""
    readonly property var record: records[recordIndex] || null
    readonly property var segments: record ? (unitMode && record.units.length ? record.units : record.segments).filter(s => TranscriptEditing.bounds(s, margin.value, trimStart, trimEnd) !== null) : []
    signal rangeRequested(real start, real end, bool play)
    signal editRequested(string kind, real start, real end)
    color: Theme.panel
    radius: 8
    border.color: Theme.border
    function refresh(): void {
        if (!asset) { records = []; return; }
        const scoped = JSON.parse(JSON.stringify(backend.selectionTranscripts(asset.id))).map(r => ({model: r.transcript.model, text: r.transcript.text, segments: r.transcript.segments, units: r.alignment ? r.alignment.items : TranscriptEditing.units(r.transcript.segments),
            label: qsTr("Selection %1–%2 s").arg((r.start_millis/1000).toFixed(2)).arg((r.end_millis/1000).toFixed(2))}));
        const full = JSON.parse(JSON.stringify(backend.transcriptsForAsset(asset.id))).map(r => ({model: r.model, text: r.text, segments: r.segments, units: [], label: qsTr("Whole recording")}));
        records = scoped.concat(full); recordIndex = 0;
    }
    function bounds(segment: var): var { return TranscriptEditing.bounds(segment,margin.value,trimStart,trimEnd); }
    function requestTranscription(): void {
        notice = "";
        if (selectionTranscription.running) selectionTranscription.discard();
        else { acceptedKey = ""; selectionTranscription.request(asset.id, Math.round(rangeStart), Math.round(rangeEnd), inferencePrefs.runtimeEndpoint, currentKey); }
    }
    onAssetChanged: refresh()
    onCurrentKeyChanged: {
        if (selectionTranscription.requestKey && selectionTranscription.requestKey !== currentKey) selectionTranscription.discard();
    }
    onVisibleChanged: { if (visible) refresh(); else if (selectionTranscription.requestKey === currentKey) selectionTranscription.discard(); }
    Component.onCompleted: refresh()
    Connections {
        target: backend
        function onAssetsChanged(): void { if (panel.visible) panel.refresh(); }
    }
    Connections {
        target: selectionTranscription
        function onResultChanged(): void {
            if (!panel.visible || !panel.asset || !selectionTranscription.resultJson || selectionTranscription.requestKey !== panel.currentKey || panel.acceptedKey === panel.currentKey) return;
            panel.acceptedKey = panel.currentKey;
            panel.notice = backend.acceptSelectionTranscript(panel.asset.id, selectionTranscription.resultJson);
            panel.refresh();
        }
    }
    ColumnLayout {
        anchors.fill: parent; anchors.margins: 14; spacing: 10
        RowLayout {
            Layout.fillWidth: true
            Label { text: qsTr("Transcript editing"); font.weight: Font.DemiBold }
            Label { Layout.fillWidth: true; text: qsTr("Original-time evidence"); color: Theme.textSecondary }
            CheckBox { text: qsTr("Aligned units"); enabled: panel.record !== null && panel.record.units.length > 0; checked: panel.unitMode; onToggled: panel.unitMode=checked }
            Label { text: qsTr("Boundary margin (ms)") }
            SpinBox { id: margin; from: 0; to: 1000; value: 80; stepSize: 10; editable: true }
            EchoButton {
                objectName: "transcribeSelectionButton"
                text: selectionTranscription.running ? qsTr("Discard result") : qsTr("Transcribe selection")
                enabled: panel.asset !== null && (!selectionTranscription.running || !selectionTranscription.discarded)
                onClicked: panel.requestTranscription()
            }
        }
        Label {
            Layout.fillWidth: true; wrapMode: Text.Wrap; color: Theme.textSecondary
            text: selectionTranscription.running ? (selectionTranscription.discarded ? qsTr("The result will be discarded. Infer Runtime may still be processing the request.") : qsTr("Transcribing the selected original audio…"))
                : selectionTranscription.errorText || panel.notice || qsTr("Select a sentence to locate or audition it. Hiding or keeping audio creates a reversible edit; it does not change the transcript.")
        }
        Label {
            visible: panel.unitMode && panel.record !== null && panel.record.units.some(unit => unit.end <= unit.start)
            Layout.fillWidth: true; wrapMode: Text.Wrap; color: Theme.textMuted
            text: qsTr("Units without a reliable duration are omitted from editing.")
        }
        EchoComboBox { Layout.fillWidth: true; visible: panel.records.length > 1; model: panel.records; textRole: "label"; selectionIndex: panel.recordIndex; onActivated: panel.recordIndex=currentIndex }
        Label { visible: panel.record !== null; text: panel.record ? panel.record.label + " · " + panel.record.model : ""; color: Theme.textMuted }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 5
            model: panel.segments
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                id: row
                required property var modelData
                readonly property var range: panel.bounds(modelData)
                width: ListView.view.width
                height: Math.max(52, sentence.implicitHeight + 20)
                radius: 5; color: Theme.panelRaised
                RowLayout {
                    anchors.fill: parent; anchors.margins: 8; spacing: 10
                    Label { text: modelData.start.toFixed(2) + " s"; color: Theme.textMuted; Layout.preferredWidth: 75 }
                    Label { id: sentence; text: row.modelData.text; Layout.fillWidth: true; wrapMode: Text.Wrap }
                    EchoButton { text: qsTr("Select"); ghost: true; enabled: row.range !== null; onClicked: panel.rangeRequested(row.range.start,row.range.end,false) }
                    EchoButton { text: qsTr("Audition"); ghost: true; enabled: row.range !== null; onClicked: panel.rangeRequested(row.range.start,row.range.end,true) }
                    EchoButton { text: qsTr("Hide audio"); ghost: true; enabled: row.range !== null; onClicked: panel.editRequested("hide",row.range.start,row.range.end) }
                    EchoButton { text: qsTr("Keep only"); ghost: true; enabled: row.range !== null; onClicked: panel.editRequested("keep",row.range.start,row.range.end) }
                }
            }
            Label { anchors.centerIn: parent; visible: !panel.segments.length; text: panel.record && panel.record.text ? qsTr("No usable sentence timing is available for this range.") : qsTr("No transcript yet. Select a range of up to five minutes and transcribe it."); color: Theme.textMuted }
        }
    }
}
