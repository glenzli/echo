//! EchoTextField: theme-styled text input (default Qt TextField looks
//! unstyled against the Echo surface palette).

import QtQuick
import QtQuick.Controls

TextField {
    id: root

    implicitHeight: Theme.controlHeight
    color: Theme.textPrimary
    placeholderTextColor: Theme.textDisabled
    selectionColor: Theme.accentSurface
    selectedTextColor: Theme.accentSelectionText
    selectByMouse: true
    padding: 10

    background: Rectangle {
        radius: Theme.controlRadius
        border.width: root.activeFocus ? 1 : 1
        border.color: root.activeFocus ? Theme.focusRing : Theme.buttonBorder
        color: Theme.control
    }
}
