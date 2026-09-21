//! A single welcome-page entry; recovery rows are virtualized in a bounded popup.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: picker
    required property var controller
    readonly property var sessions: controller.recoverableSessions || []
    property alias dialog: recoveryDialog
    spacing: 0
    visible: sessions.length > 0
    EchoButton {
        objectName: "editorRecoveryButton"
        Layout.alignment: Qt.AlignHCenter
        text: qsTr("Recover previous editing… (%1)").arg(picker.sessions.length)
        ghost: true
        onClicked: { picker.controller.refreshRecoverableSessions(); recoveryDialog.open(); }
    }
    Popup {
        id: recoveryDialog
        parent: Overlay.overlay
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: Math.min(540, parent.width - 32)
        height: Math.min(460, parent.height - 32)
        padding: 20; modal: true; dim: true; focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        background: Rectangle { color: Theme.panelRaised; radius: Theme.controlRadius + 3; border.color: Theme.borderStrong }
        contentItem: ColumnLayout {
            spacing: 12
            Text { text: qsTr("Recover previous editing"); color: Theme.textPrimary; font.pixelSize: 18; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; text: qsTr("Choose an unfinished project to open in another window."); wrapMode: Text.WordWrap; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            ListView {
                id: sessionsList
                objectName: "editorRecoveryList"
                Layout.fillWidth: true; Layout.fillHeight: true
                clip: true; spacing: 6
                model: picker.sessions
                ScrollBar.vertical: ScrollBar {}
                delegate: ItemDelegate {
                    id: row
                    required property var modelData
                    required property int index
                    objectName: "editorRecoveryRow-" + index
                    width: sessionsList.width; height: 66
                    Accessible.name: modelData.title + " · " + row.dateText
                    readonly property string dateText: new Date(Number(modelData.modifiedMillis)).toLocaleString(Qt.locale(), Locale.ShortFormat)
                    onClicked: if (picker.controller.resumeSession(modelData.path)) recoveryDialog.close()
                    background: Rectangle { color: row.hovered || row.activeFocus ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle; radius: Theme.controlRadius; border.color: row.activeFocus ? Theme.focusRing : Theme.border }
                    contentItem: ColumnLayout {
                        spacing: 4
                        Text { Layout.fillWidth: true; text: row.modelData.title; textFormat: Text.PlainText; elide: Text.ElideMiddle; color: Theme.textPrimary; font.pixelSize: Theme.fontBody }
                        Text { Layout.fillWidth: true; text: qsTr("%1 sources · %2").arg(row.modelData.sourceCount).arg(row.dateText); elide: Text.ElideRight; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                    }
                }
            }
            Text { visible: !picker.sessions.length; text: qsTr("No unfinished projects are available."); color: Theme.textMuted; font.pixelSize: Theme.fontBody }
            EchoButton { Layout.alignment: Qt.AlignRight; text: qsTr("Close"); ghost: true; onClicked: recoveryDialog.close() }
        }
    }
}
