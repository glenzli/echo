//! Reviewable user declarations in immutable Original coordinates.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SourceDisclosure.js" as Disclosure
Popup {
    id: dialog
    required property var catalogBackend
    property var asset: null
    property real rangeStart: 0
    property real rangeEnd: 0
    property var spans: []
    property real expectedRevision: 0
    property string errorText: ""
    readonly property bool importedLabels: Disclosure.sources(asset).some(source => source.origin === "embedded_export")
    readonly property bool readOnly: !!(asset && asset.assemblyId)
    property alias kindIndex: kind.currentIndex
    property alias noteText: note.text
    property alias labelList: list
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(600,parent.width-32); height: Math.min(580,parent.height-32)
    padding: 20; modal: true; dim: true; focus: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { color: Theme.panelRaised; radius: 12; border.color: Theme.border }
    function kindName(value) {
        return value === "ai_generated" ? qsTr("Generated addition") : value === "reconstructed_speech" ? qsTr("Reconstructed speech") : qsTr("AI processing");
    }
    function present(source, start, end) {
        asset=source; rangeStart=Math.max(0,Math.floor(start)); rangeEnd=Math.min(Number(source.durationMillis),Math.ceil(end));
        const revision=Disclosure.ownRevision(source); expectedRevision=revision.revisionId;
        spans=JSON.parse(JSON.stringify(readOnly ? Disclosure.sources(source).reduce((rows,s) => rows.concat(Disclosure.list(s.spans).map(p => Object.assign({},p,{sourceId:s.assetId}))),[]) : revision.spans));
        note.text=""; errorText=""; kind.currentIndex=0; open();
    }
    function append(whole) {
        if (!asset || readOnly || spans.length>=64) return;
        const start=whole?0:rangeStart, end=whole?Number(asset.durationMillis):rangeEnd;
        if(end<=start) return;
        spans=spans.concat([{kind:["ai_generated","ai_processed","reconstructed_speech"][kind.currentIndex],startMillis:start,endMillis:end,note:note.text.trim()}]);
        note.text="";
    }
    function save() {
        if(readOnly || !asset) return;
        errorText=catalogBackend.setSourceDisclosure(asset.id,expectedRevision,spans);
        if(!errorText) close();
    }
    contentItem: ColumnLayout {
        spacing: 12
        Text { text: qsTr("Source labels"); font.pixelSize: 18; font.weight: Font.DemiBold; color: Theme.textPrimary }
        Text {
            Layout.fillWidth: true; wrapMode: Text.WordWrap; font.pixelSize: Theme.fontBody; color: Theme.textSecondary
            text: dialog.readOnly ? qsTr("Source declarations for this retained mix. Open a source to correct its labels.") : qsTr("Declare AI processing or generated additions in this source. This changes labels only; the original audio stays intact.")
        }
        Text {
            Layout.fillWidth: true; visible: dialog.importedLabels; wrapMode: Text.WordWrap
            font.pixelSize: Theme.fontMeta; color: Theme.textSecondary
            text: qsTr("These labels came from the audio file and apply to its whole duration. They are declarations, not authenticated evidence; you can correct them.")
        }
        Rectangle { Layout.fillWidth: true; implicitHeight: disclaimer.implicitHeight+16; radius: 7; color: Theme.warningSurface
            Text { id: disclaimer; anchors.fill: parent; anchors.margins: 8; wrapMode: Text.WordWrap; font.pixelSize: Theme.fontMeta; color: Theme.warningText
                text: qsTr("Unmarked does not mean verified. Reconstructed speech must not be treated as recorded dialogue. Corrections retain their history.") }
        }
        RowLayout {
            visible: !dialog.readOnly; Layout.fillWidth: true
            EchoComboBox { id: kind; objectName: "disclosureKind"; Layout.preferredWidth: 190; model: [qsTr("Generated addition"),qsTr("AI processing"),qsTr("Reconstructed speech")] }
            EchoTextField { id: note; objectName: "disclosureNote"; Layout.fillWidth: true; maximumLength: 240; placeholderText: qsTr("What was added or processed?") }
        }
        RowLayout {
            visible: !dialog.readOnly; Layout.fillWidth: true
            EchoButton { objectName: "disclosureAddSelection"; text: qsTr("Label selection"); enabled: dialog.rangeEnd>dialog.rangeStart && dialog.spans.length<64; onClicked: dialog.append(false) }
            EchoButton { objectName: "disclosureAddWhole"; text: qsTr("Label whole source"); ghost: true; enabled: dialog.asset && dialog.asset.durationMillis>0 && dialog.spans.length<64; onClicked: dialog.append(true) }
            Item { Layout.fillWidth: true }
            Text { text: qsTr("%1 / 64 labels").arg(dialog.spans.length); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
        }
        ListView {
            id: list; Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 6; model: dialog.spans
            ScrollBar.vertical: ScrollBar {}
            Text { anchors.centerIn: parent; visible: dialog.spans.length===0; text: qsTr("No source labels yet"); color: Theme.textMuted; font.pixelSize: Theme.fontBody }
            delegate: Rectangle {
                required property var modelData; required property int index
                width: list.width; height: details.implicitHeight+18; radius: 6; color: Theme.surfaceSubtle
                RowLayout { anchors.fill: parent; anchors.margins: 9; spacing: 8
                    ColumnLayout { id: details; Layout.fillWidth: true; spacing: 3
                        Text { Layout.fillWidth: true; text: dialog.kindName(modelData.kind)+" · "+(modelData.startMillis/1000).toFixed(3)+"–"+(modelData.endMillis/1000).toFixed(3)+" s"; color: Theme.textPrimary; font.pixelSize: Theme.fontSection; elide: Text.ElideRight }
                        Text { Layout.fillWidth: true; visible: modelData.note.length>0; text: modelData.note; wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
                        Text { Layout.fillWidth: true; visible: dialog.readOnly; text: modelData.sourceId || ""; elide: Text.ElideMiddle; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                    }
                    EchoButton { objectName: "disclosureRemove"+index; visible: !dialog.readOnly; text: "×"; Accessible.name: qsTr("Remove label"); implicitWidth: 28; implicitHeight: 28; ghost: true; onClicked: dialog.spans=dialog.spans.filter((_,i)=>i!==index) }
                }
            }
        }
        Text { Layout.fillWidth: true; visible: !!dialog.errorText; text: dialog.errorText; wrapMode: Text.WordWrap; color: Theme.warningText; font.pixelSize: Theme.fontBody }
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: qsTr("Times refer to the original source."); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            EchoButton { text: dialog.readOnly ? qsTr("Close") : qsTr("Cancel"); ghost: true; onClicked: dialog.close() }
            EchoButton { objectName: "disclosureSave"; visible: !dialog.readOnly; text: qsTr("Save labels"); onClicked: dialog.save() }
        }
    }
}
