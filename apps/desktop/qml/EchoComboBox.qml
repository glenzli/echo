//! Theme-aligned selector shared by the source bin and project inspector.
import QtQuick
import QtQuick.Controls

ComboBox {
    id: control
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontBody
    leftPadding: 10
    rightPadding: 28
    contentItem: Text {
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.textPrimary : Theme.textDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    indicator: Text {
        x: control.width - width - 10
        y: (control.height - height) / 2
        text: "⌄"
        color: Theme.textMuted
        font.pixelSize: 16
    }
    background: Rectangle {
        radius: Theme.controlRadius
        color: control.down ? Theme.controlPressed : Theme.control
        border.color: control.activeFocus ? Theme.focusRing : Theme.borderStrong
    }
}
