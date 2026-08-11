//! Shared visual header for the two sibling adjustment surfaces.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

RowLayout {
    id: header

    required property string title
    required property string iconSource

    implicitHeight: 40
    Layout.leftMargin: 12
    Layout.rightMargin: 10
    spacing: 7

    EchoIcon {
        source: header.iconSource
        size: 14
        color: Theme.textSecondary
    }

    Text {
        text: header.title
        color: Theme.textPrimary
        font.pixelSize: Theme.fontBody
        font.weight: Font.DemiBold
    }

    Item {
        Layout.fillWidth: true
    }
}
