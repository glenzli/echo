//! Compact detail-inspector section with a consistent heading and body rhythm.

import QtQuick
import QtQuick.Layouts
import EchoDesktop

ColumnLayout {
    id: section

    required property string title
    default property alias content: body.data

    spacing: 8

    Rectangle {
        Layout.fillWidth: true
        Layout.leftMargin: 16
        Layout.rightMargin: 16
        Layout.preferredHeight: 1
        color: Theme.border
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: 16
        Layout.rightMargin: 16

        Text {
            Layout.fillWidth: true
            text: section.title
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            font.bold: true
            font.letterSpacing: 1.0
        }

    }

    ColumnLayout {
        id: body
        Layout.fillWidth: true
        Layout.leftMargin: 16
        Layout.rightMargin: 16
        spacing: 6
    }
}
