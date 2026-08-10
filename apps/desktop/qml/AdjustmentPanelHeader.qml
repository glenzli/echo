//! Shared visual header for the two sibling adjustment surfaces.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

RowLayout {
    id: header

    required property string title
    required property string iconSource

    implicitHeight: 30
    Layout.leftMargin: 10
    Layout.rightMargin: 8
    spacing: 6

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

    Item { Layout.fillWidth: true }
}
