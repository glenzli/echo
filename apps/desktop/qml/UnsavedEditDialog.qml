//! Guards navigation away from a single-source draft without publishing it implicitly.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
Popup {
    id: dialog
    required property var editor
    property var pendingAction: null
    property bool saveFailed: false
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(480,parent.width-32); padding: 20
    modal: true; dim: true; focus: true
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { radius: 12; color: Theme.panelRaised; border.color: Theme.border }
    function request(action) {
        if (visible) return;
        if (!editor.dirty) { action(); return; }
        pendingAction=action; saveFailed=false; open();
    }
    function proceed() {
        const action=pendingAction;
        pendingAction=null; close();
        if (action) action();
    }
    onClosed: pendingAction=null
    contentItem: ColumnLayout {
        spacing: 16
        Text { Layout.fillWidth: true; text: qsTr("Keep these edits before leaving?"); wrapMode: Text.WordWrap; font.pixelSize: 18; font.weight: Font.DemiBold; color: Theme.textPrimary }
        Text { Layout.fillWidth: true; text: qsTr("This sound has unsaved adjustments. Saving keeps this editing version; the original file stays unchanged."); wrapMode: Text.WordWrap; font.pixelSize: Theme.fontBody; color: Theme.textSecondary }
        Text { Layout.fillWidth: true; visible: dialog.saveFailed; text: qsTr("The edits could not be saved. Keep editing or try again."); wrapMode: Text.WordWrap; font.pixelSize: Theme.fontBody; color: Theme.warningText }
        RowLayout {
            Layout.fillWidth: true
            EchoButton { objectName: "keepEditing"; text: qsTr("Keep editing"); ghost: true; onClicked: dialog.close() }
            Item { Layout.fillWidth: true }
            EchoButton { objectName: "discardEdits"; text: qsTr("Discard edits"); ghost: true; onClicked: { dialog.editor.adjustment.revert(); dialog.proceed(); } }
            EchoButton { objectName: "saveEdits"; text: qsTr("Save and continue"); onClicked: { dialog.editor.save(); if (dialog.editor.dirty) dialog.saveFailed=true; else dialog.proceed(); } }
        }
    }
}
