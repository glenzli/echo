//! Review AI time evidence; accepted audio changes belong to the source draft.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SoundTranscriptEditing.js" as TranscriptEditing
import "SourceEditRanges.js" as SourceEditRanges

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
    property bool gapMode: false
    property string searchText: ""
    property int boundaryMargin: 80
    property int minimumGap: 600
    property var selectedKeys: []
    property string notice: ""
    property string editError: ""
    property string acceptedKey: ""
    readonly property string assetKey: asset ? asset.id : ""
    readonly property string currentKey: asset ? asset.id + ":" + revisionKey + ":" + rangeStart + ":" + rangeEnd : ""
    readonly property var record: records[recordIndex] || null
    readonly property bool textOnly: record !== null && record.text.length > 0 && !record.segments.concat(record.units).some(s => Number.isFinite(s.start) && Number.isFinite(s.end) && s.start >= 0 && s.end > s.start)
    readonly property bool usingUnits: unitMode && record !== null && record.units.length > 0
    readonly property var candidates: !record ? [] : gapMode
        ? TranscriptEditing.gaps(record.units, minimumGap, boundaryMargin)
        : TranscriptEditing.entries(usingUnits ? record.units : record.segments)
    readonly property var segments: TranscriptEditing.filtered(candidates, searchText, usingUnits && !gapMode)
        .filter(s => bounds(s) !== null)
    readonly property var selectedLookup: { const keys = {}; for (const key of selectedKeys) keys[key] = true; return keys; }
    readonly property var selectedEntries: segments.filter(s => selectedLookup[s.key] === true)
    readonly property var selectedRanges: SourceEditRanges.normalized(selectedEntries.map(s => bounds(s)),trimStart,trimEnd) || []
    readonly property real selectedDuration: selectedRanges.reduce((sum,r) => sum+r.end-r.start,0)
    signal rangeRequested(real start, real end, bool play)
    signal editsRequested(string kind, var ranges)
    color: Theme.panel
    radius: 8
    border.color: Theme.border

    function clearSelection(): void { selectedKeys = []; editError = ""; }
    function toggleEntry(key: string): void {
        selectedKeys = selectedKeys.indexOf(key) === -1 ? selectedKeys.concat([key]) : selectedKeys.filter(k => k !== key);
        editError = "";
    }
    function selectVisible(): void { selectedKeys = segments.map(s => s.key); editError = ""; }
    function refresh(): void {
        if (!asset) { records = []; return; }
        const scoped = JSON.parse(JSON.stringify(backend.selectionTranscripts(asset.id))).map(r => ({model: r.transcript.model, text: r.transcript.text, segments: r.transcript.segments, units: r.alignment ? r.alignment.items : TranscriptEditing.units(r.transcript.segments),
            label: qsTr("Selection %1–%2 s").arg((r.start_millis/1000).toFixed(2)).arg((r.end_millis/1000).toFixed(2))}));
        const full = JSON.parse(JSON.stringify(backend.transcriptsForAsset(asset.id))).map(r => ({model: r.model, text: r.text, segments: r.segments, units: TranscriptEditing.units(r.segments), label: qsTr("Whole recording")}));
        const next = scoped.concat(full);
        if (JSON.stringify(next) === JSON.stringify(records)) return;
        clearSelection(); records = next; recordIndex = 0;
    }
    function bounds(segment: var): var { return TranscriptEditing.bounds(segment,segment.kind === "gap" ? 0 : boundaryMargin,trimStart,trimEnd); }
    function locate(segment: var, play: bool): void {
        const range = bounds(segment);
        if (!range) return;
        const context = play && segment.kind === "gap" ? 250 : 0;
        rangeRequested(Math.max(trimStart,range.start-context),Math.min(trimEnd,range.end+context),play);
    }
    function applySelection(kind: string): void {
        if (selectedRanges.length) editsRequested(kind, selectedRanges);
    }
    function finishEdit(result: var): void {
        if (result.ok) { clearSelection(); return; }
        editError = result.error === "limit" ? qsTr("This edit would exceed 128 source segments. Select fewer ranges.")
            : result.error === "empty" ? qsTr("Keep some audio or an inserted gap. No changes were made.")
            : qsTr("These ranges are no longer available. Select them again.");
    }
    function requestTranscription(): void {
        notice = "";
        if (selectionTranscription.running) selectionTranscription.discard();
        else { acceptedKey = ""; selectionTranscription.request(asset.id, Math.round(rangeStart), Math.round(rangeEnd), inferencePrefs.runtimeEndpoint, currentKey); }
    }
    onAssetChanged: refresh()
    onAssetKeyChanged: clearSelection()
    onRecordsChanged: clearSelection()
    onRevisionKeyChanged: clearSelection()
    onRecordIndexChanged: clearSelection()
    onUnitModeChanged: clearSelection()
    onGapModeChanged: { searchText = ""; clearSelection(); }
    onSearchTextChanged: clearSelection()
    onBoundaryMarginChanged: clearSelection()
    onMinimumGapChanged: clearSelection()
    onTrimStartChanged: clearSelection()
    onTrimEndChanged: clearSelection()
    onCurrentKeyChanged: {
        if (selectionTranscription.requestKey && selectionTranscription.requestKey !== currentKey) selectionTranscription.discard();
    }
    onVisibleChanged: { if (visible) refresh(); else { clearSelection(); if (selectionTranscription.requestKey === currentKey) selectionTranscription.discard(); } }
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
        anchors.fill: parent; anchors.margins: 12; spacing: 8
        RowLayout {
            Layout.fillWidth: true; spacing: 10
            Label { text: qsTr("Transcript editing"); font.weight: Font.DemiBold; font.pixelSize: Theme.fontSection }
            EchoComboBox { Layout.fillWidth: true; visible: panel.records.length > 1; model: panel.records; textRole: "label"; selectionIndex: panel.recordIndex; onActivated: panel.recordIndex=currentIndex }
            Label { Layout.fillWidth: true; visible: panel.records.length <= 1; text: panel.record ? panel.record.label : qsTr("Original-time evidence"); color: Theme.textMuted; elide: Text.ElideRight }
            TranscriptExportMenu {
                id: transcriptDelivery
                record: panel.record; sourcePath: panel.asset ? String(panel.asset.path || "") : ""
            }
            EchoButton {
                objectName: "transcribeSelectionButton"
                Layout.preferredWidth: 180
                text: selectionTranscription.running ? qsTr("Discard result") : qsTr("Transcribe selection")
                enabled: panel.asset !== null && (!selectionTranscription.running || !selectionTranscription.discarded)
                onClicked: panel.requestTranscription()
            }
        }
        RowLayout {
            Layout.fillWidth: true; spacing: 10
            EchoSegmentedControl {
                objectName: "transcriptMode"
                model: [qsTr("Text"),qsTr("Speech gaps")]; currentIndex: panel.gapMode ? 1 : 0
                onActivated: index => panel.gapMode=index===1
            }
            EchoTextField {
                objectName: "transcriptSearch"
                Layout.fillWidth: true; visible: !panel.gapMode
                placeholderText: qsTr("Find words or phrases…")
                text: panel.searchText; onTextEdited: panel.searchText=text
            }
            EchoCheckBox {
                objectName: "transcriptUnits"
                visible: !panel.gapMode; text: qsTr("Aligned units")
                enabled: panel.record !== null && panel.record.units.length > 0
                checked: panel.usingUnits; onToggled: panel.unitMode=checked
            }
            Label { visible: panel.gapMode; text: qsTr("Minimum gap (ms)"); color: Theme.textSecondary }
            EchoValueSpinBox {
                objectName: "minimumSpeechGap"
                visible: panel.gapMode; from: 200; to: 10000; value: panel.minimumGap; stepSize: 100
                onValueModified: panel.minimumGap=value
            }
            Item { visible: panel.gapMode; Layout.fillWidth: true }
            Label { text: panel.gapMode ? qsTr("Retain at each edge (ms)") : qsTr("Boundary margin (ms)"); color: Theme.textSecondary }
            EchoValueSpinBox {
                objectName: "transcriptMargin"
                from: 0; to: 1000; value: panel.boundaryMargin; stepSize: 10
                onValueModified: panel.boundaryMargin=value
            }
        }
        Label {
            Layout.fillWidth: true; wrapMode: Text.Wrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta
            text: selectionTranscription.running ? (selectionTranscription.discarded ? qsTr("The result will be discarded. Infer Runtime may still be processing the request.") : qsTr("Transcribing the selected original audio…"))
                : selectionTranscription.errorText || panel.notice || transcriptDelivery.notice || (panel.textOnly
                    ? qsTr("This transcript has no timing. You can read, copy, or export the text; timed audio edits are unavailable.") : panel.gapMode
                    ? qsTr("Gaps between aligned speech may contain ambience. Audition includes 250 ms of context; only the marked interval is edited.")
                    : qsTr("Search and check sentences or aligned units. Audition plays the original; batch edits are reversible and keep the transcript intact."))
        }
        Label {
            Layout.fillWidth: true; visible: panel.record !== null; wrapMode: Text.Wrap
            text: qsTr("Exports include this complete AI transcript. Subtitle times refer to the original recording, before audio edits.")
            color: Theme.textMuted; font.pixelSize: Theme.fontMeta
        }
        ScrollView {
            Layout.fillWidth: true; Layout.fillHeight: true; visible: panel.textOnly && !panel.gapMode; clip: true
            TextArea {
                objectName: "untimedTranscript"
                text: panel.record ? panel.record.text : ""; textFormat: Text.PlainText
                readOnly: true; selectByMouse: true; wrapMode: Text.Wrap
                color: Theme.textPrimary; font.pixelSize: Theme.fontBody
                background: Rectangle { color: Theme.panelRaised; radius: 5 }
            }
        }
        ListView {
            id: list
            visible: !panel.textOnly || panel.gapMode
            objectName: "transcriptRows"
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 4
            model: panel.segments; reuseItems: true
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                id: row
                required property var modelData
                readonly property bool selected: panel.selectedLookup[modelData.key] === true
                width: ListView.view.width
                height: Math.max(44, sentence.implicitHeight + 16)
                radius: 5; color: selected ? Theme.accentSurfaceQuiet : Theme.panelRaised
                border.color: selected ? Theme.accent : Theme.transparent
                RowLayout {
                    anchors.fill: parent; anchors.margins: 6; spacing: 8
                    EchoCheckBox {
                        objectName: "check-"+row.modelData.key
                        checked: row.selected; onToggled: panel.toggleEntry(row.modelData.key)
                        Accessible.name: qsTr("Select %1").arg(row.modelData.text)
                    }
                    Label {
                        text: row.modelData.start.toFixed(2) + "–" + row.modelData.end.toFixed(2) + " s"
                        font.family: "Menlo"; font.pixelSize: Theme.fontMeta
                        color: Theme.textMuted; Layout.preferredWidth: 136
                    }
                    Label {
                        id: sentence; text: row.modelData.text; textFormat: Text.PlainText
                        Layout.fillWidth: true; wrapMode: Text.Wrap; color: Theme.textPrimary
                        TapHandler { onTapped: panel.toggleEntry(row.modelData.key) }
                    }
                    EchoButton { text: qsTr("Locate"); ghost: true; onClicked: panel.locate(row.modelData,false) }
                    EchoButton { objectName: "audition-"+row.modelData.key; text: qsTr("Audition source"); ghost: true; onClicked: panel.locate(row.modelData,true) }
                }
            }
            Label {
                anchors.centerIn: parent; width: Math.max(0,parent.width-32); horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                visible: !panel.segments.length; color: Theme.textMuted
                text: !panel.record ? qsTr("No transcript yet. Select a range of up to five minutes and transcribe it.")
                    : panel.gapMode ? (panel.record.units.length ? qsTr("No speech gaps meet these settings.") : qsTr("Speech gaps need aligned timing. Transcribe a selection to obtain it."))
                    : panel.searchText ? qsTr("No matching text in this range.") : qsTr("No usable timing is available. Units without a reliable duration cannot be edited.")
            }
        }
        Label { visible: panel.editError.length>0; text: panel.editError; Layout.fillWidth: true; wrapMode: Text.Wrap; color: Theme.textPrimary }
        RowLayout {
            Layout.fillWidth: true; spacing: 8
            EchoButton { objectName: "selectTranscriptResults"; text: qsTr("Select results"); ghost: true; enabled: panel.segments.length>0; onClicked: panel.selectVisible() }
            EchoButton { objectName: "clearTranscriptSelection"; text: qsTr("Clear"); ghost: true; enabled: panel.selectedEntries.length>0; onClicked: panel.clearSelection() }
            Label { Layout.fillWidth: true; text: qsTr("%1 selected · %2 s").arg(panel.selectedEntries.length).arg((panel.selectedDuration/1000).toFixed(2)); color: Theme.textSecondary }
            EchoButton { objectName: "keepTranscriptSelection"; visible: !panel.gapMode; text: qsTr("Keep selected"); ghost: true; enabled: panel.selectedRanges.length>0; onClicked: panel.applySelection("keep") }
            EchoButton { objectName: "hideTranscriptSelection"; text: qsTr("Hide selected"); enabled: panel.selectedRanges.length>0; onClicked: panel.applySelection("hide") }
        }
    }
}
