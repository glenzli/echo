//! Compact numeric input with the same typography and surfaces as track controls.
import QtQuick
import QtQuick.Controls

SpinBox {
    id: control
    implicitWidth: 128
    implicitHeight: 26
    editable: true
    font.pixelSize: Theme.fontSection
    font.family: "Menlo"
    padding: 0
    leftPadding: 26
    rightPadding: 26
    contentItem: TextInput {
        objectName: "numericInput"
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.textPrimary : Theme.textDisabled
        selectionColor: Theme.accent
        selectedTextColor: Theme.accentText
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        readOnly: !control.editable
        validator: control.validator
        inputMethodHints: control.inputMethodHints
        clip: true
        selectByMouse: true
    }
    background: Rectangle {
        radius: 5
        color: control.enabled ? Theme.control : Theme.controlQuiet
        border.color: control.activeFocus ? Theme.focusRing : Theme.border
    }
    up.indicator: Rectangle {
        x: control.width - width
        width: 24; height: control.height; radius: 5
        color: control.up.pressed ? Theme.buttonGhostPressed : control.up.hovered ? Theme.buttonGhostHover : Theme.transparent
        Text { anchors.centerIn: parent; text: "+"; font.pixelSize: 14; color: control.enabled ? Theme.textSecondary : Theme.textDisabled }
    }
    down.indicator: Rectangle {
        width: 24; height: control.height; radius: 5
        color: control.down.pressed ? Theme.buttonGhostPressed : control.down.hovered ? Theme.buttonGhostHover : Theme.transparent
        Text { anchors.centerIn: parent; text: "−"; font.pixelSize: 14; color: control.enabled ? Theme.textSecondary : Theme.textDisabled }
    }
}
