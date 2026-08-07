//! EchoButton: the first `Echo*` component of the series design language.
//! Visual tokens stay consistent with Shadow's component set.

import QtQuick
import QtQuick.Controls

Button {
    id: root

    property color backgroundColor: Theme.accent
    property color textColor: Theme.accentText

    implicitHeight: 32
    implicitWidth: Math.max(72, implicitContentWidth + 24)
    padding: 8

    background: Rectangle {
        radius: 6
        color: root.down ? Qt.darker(root.backgroundColor, 1.15)
                         : root.hovered ? Qt.lighter(root.backgroundColor, 1.1)
                                        : root.backgroundColor
    }

    contentItem: Text {
        text: root.text
        color: root.textColor
        font.pixelSize: 13
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
