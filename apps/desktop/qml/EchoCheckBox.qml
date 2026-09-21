//! Compact checked selection with Echo's shared surface and focus colors.
import QtQuick
import QtQuick.Controls

CheckBox {
    id: control
    implicitHeight: Theme.controlHeight
    implicitWidth: text.length ? implicitContentWidth + leftPadding + rightPadding : 28
    padding: 4
    leftPadding: text.length ? 26 : 4
    font.pixelSize: Theme.fontBody
    indicator: Rectangle {
        x: control.text.length ? 4 : (control.width-width)/2
        y: (control.height-height)/2
        width: 16; height: 16; radius: 4
        color: control.checked ? Theme.accent : control.hovered ? Theme.buttonGhostHover : Theme.control
        border.width: control.activeFocus ? 2 : 1
        border.color: control.activeFocus ? Theme.focusRing : control.checked ? Theme.accent : Theme.borderStrong
        opacity: control.enabled ? 1 : 0.5
        Text {
            anchors.centerIn: parent; text: "✓"; visible: control.checked
            color: Theme.accentText; font.pixelSize: 12; font.weight: Font.DemiBold
        }
    }
    contentItem: Text {
        text: control.text; font: control.font; textFormat: Text.PlainText
        color: control.enabled ? Theme.textPrimary : Theme.textDisabled
        verticalAlignment: Text.AlignVCenter
    }
}
