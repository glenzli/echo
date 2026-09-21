//! Personal context editor. Draft text stays here until an explicit Catalog save.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: dialog
    required property var catalogBackend
    property string targetId: ""
    property bool assembly: false
    property double revision: 0
    property var moments: []
    property double sourceStart: -1
    property double sourceEnd: -1
    property string errorText: ""
    property alias notes: notesInput.text
    property alias place: placeInput.text
    property alias timeDescription: timeInput.text
    signal saved()

    parent: Overlay.overlay
    width: Math.min(540, parent.width - 32)
    height: Math.min(680, parent.height - 32)
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    padding: 20
    modal: true
    focus: true
    closePolicy: Popup.NoAutoClose
    background: Rectangle { color: Theme.panelRaised; radius: Theme.controlRadius + 3; border.color: Theme.borderStrong }

    function present(id, isAssembly, start = -1, end = -1) {
        targetId = id; assembly = isAssembly; sourceStart = start; sourceEnd = end;
        const result = catalogBackend.memoryInfo(id, isAssembly);
        errorText = result.error || "";
        revision = result.revision || 0;
        const info = result.info || {};
        notes = info.notes || ""; place = info.place || ""; timeDescription = info.timeDescription || "";
        moments = JSON.parse(JSON.stringify(info.moments || []));
        saveButton.enabled = !result.error;
        open();
    }
    function save(): void {
        if (!saveButton.enabled) return;
        if (notes.length > 2000 || place.length > 200 || timeDescription.length > 120) {
            errorText = qsTr("Use at most 2,000 characters for notes, 200 for place and 120 for time."); return;
        }
        if (moments.some(item => !item.note.trim() || item.note.length > 500)) {
            errorText = qsTr("Each moment needs a note of 1–500 characters."); return;
        }
        errorText = catalogBackend.setMemoryInfo(targetId, assembly, revision, {
            notes: notes, place: place, timeDescription: timeDescription, moments: moments
        });
        if (!errorText) { saved(); close(); }
    }
    function addMoment(): void {
        if (assembly || sourceStart < 0 || moments.length >= 128) return;
        moments = moments.concat([{startMillis: Math.round(sourceStart), endMillis: sourceEnd > sourceStart ? Math.round(sourceEnd) : null, note: ""}]);
    }
    function position(value: real): string {
        return Math.floor(value / 60000) + ":" + String(Math.floor(value / 1000) % 60).padStart(2, "0") + "." + String(Math.round(value % 1000)).padStart(3, "0");
    }
    component NoteArea: TextArea {
        Layout.fillWidth: true
        color: Theme.textPrimary; placeholderTextColor: Theme.textDisabled
        selectionColor: Theme.accentSurface; selectedTextColor: Theme.accentSelectionText
        font.pixelSize: Theme.fontBody; selectByMouse: true; wrapMode: TextEdit.Wrap; padding: 10
        background: Rectangle { color: Theme.control; radius: Theme.controlRadius; border.color: parent.activeFocus ? Theme.focusRing : Theme.buttonBorder }
    }
    contentItem: ColumnLayout {
        spacing: 14
        Text { text: qsTr("Memory information"); color: Theme.textPrimary; font.pixelSize: Theme.fontSection; font.weight: Font.DemiBold }
        Text {
            Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.textMuted; font.pixelSize: Theme.fontMeta
            text: qsTr("Your notes stay in Echo or this project. Original files and AI descriptions are unchanged.")
        }
        ScrollView {
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: parent.width; spacing: 10
                Text { text: qsTr("Notes"); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
                NoteArea { id: notesInput; objectName: "memoryNotes"; Layout.preferredHeight: 130; placeholderText: qsTr("What happened, and why you kept this sound…"); Accessible.name: qsTr("Notes") }
                Text { text: qsTr("Place"); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
                EchoTextField { id: placeInput; objectName: "memoryPlace"; Layout.fillWidth: true; maximumLength: 200; placeholderText: qsTr("For example, Grandma’s balcony"); Accessible.name: qsTr("Place") }
                Text { text: qsTr("Time"); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
                EchoTextField { id: timeInput; objectName: "memoryTime"; Layout.fillWidth: true; maximumLength: 120; placeholderText: qsTr("An exact date or roughly when it happened"); Accessible.name: qsTr("Time") }
                Text {
                    visible: !dialog.assembly; Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: qsTr("Moment notes refer to the original recording. Add one from the editor’s playhead or selected range.")
                    color: Theme.textMuted; font.pixelSize: Theme.fontMeta
                }
                EchoButton { visible: !dialog.assembly && dialog.sourceStart >= 0; enabled: dialog.moments.length < 128; text: qsTr("Add note at %1").arg(dialog.position(dialog.sourceStart)); ghost: true; onClicked: dialog.addMoment() }
                Repeater {
                    model: dialog.moments
                    delegate: ColumnLayout {
                        id: momentRow
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        RowLayout {
                            Text { Layout.fillWidth: true; text: dialog.position(momentRow.modelData.startMillis) + (momentRow.modelData.endMillis !== null ? " – " + dialog.position(momentRow.modelData.endMillis) : ""); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
                            EchoButton { text: qsTr("Remove"); ghost: true; onClicked: dialog.moments = dialog.moments.filter((_, i) => i !== momentRow.index) }
                        }
                        NoteArea {
                            Layout.preferredHeight: 70
                            text: momentRow.modelData.note
                            Accessible.name: qsTr("Moment note")
                            onTextChanged: momentRow.modelData.note = text
                        }
                    }
                }
            }
        }
        Text { visible: !!dialog.errorText; text: dialog.errorText; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.warningText; font.pixelSize: Theme.fontMeta }
        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            EchoButton { text: qsTr("Cancel"); ghost: true; onClicked: dialog.close() }
            EchoButton { id: saveButton; objectName: "saveMemoryInfo"; text: qsTr("Save"); onClicked: dialog.save() }
        }
    }
}
