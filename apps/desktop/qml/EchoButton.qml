//! EchoButton: text button for dialogs and primary actions, using theme
//! tokens. Toolbar actions prefer EchoIconButton.

import QtQuick
import QtQuick.Controls

Button {
    id: root

    property color backgroundColor: Theme.accent
    property color textColor: Theme.accentText
    property bool ghost: false

    implicitHeight: Theme.controlHeight
    implicitWidth: Math.max(72, implicitContentWidth + 24)
    padding: 8

    background: Rectangle {
        radius: Theme.controlRadius
        border.width: root.ghost ? 1 : 0
        border.color: Theme.buttonBorder
        color: {
            if (!root.enabled) {
                return Theme.controlQuiet
            }
            if (root.ghost) {
                return root.down ? Theme.buttonGhostPressed
                                 : root.hovered ? Theme.buttonGhostHover
                                                : Theme.transparent
            }
            return root.down ? Qt.darker(root.backgroundColor, 1.15)
                             : root.hovered ? Qt.lighter(root.backgroundColor, 1.1)
                                            : root.backgroundColor
        }
    }

    contentItem: Text {
        text: root.text
        color: !root.enabled ? Theme.textDisabled
                             : root.ghost ? Theme.textPrimary : root.textColor
        font.pixelSize: Theme.fontBody
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
