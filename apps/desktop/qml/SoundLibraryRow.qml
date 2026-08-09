//! One reusable Library navigation row.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: row

    required property string label
    required property string glyph
    required property int count
    property bool selected: false
    property string subtitle: ""

    signal activated()

    implicitHeight: subtitle.length > 0 ? 46 : 34
    radius: Theme.controlRadius
    color: selected ? Theme.accentSurfaceQuiet
                    : hover.hovered ? Theme.surfaceSubtle : Theme.transparent

    Rectangle {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        width: 3
        height: 20
        radius: 1.5
        visible: row.selected
        color: Theme.accent
    }

    HoverHandler { id: hover }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: row.activated()
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 8
        spacing: 8

        Text {
            Layout.preferredWidth: 17
            text: row.glyph
            color: row.selected ? Theme.accentSelectionText : Theme.textSecondary
            font.pixelSize: 14
            horizontalAlignment: Text.AlignHCenter
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Text {
                Layout.fillWidth: true
                text: row.label
                color: row.selected ? Theme.accentSelectionText : Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.bold: row.selected
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                visible: row.subtitle.length > 0
                text: row.subtitle
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        Text {
            text: String(row.count)
            color: Theme.textDisabled
            font.pixelSize: Theme.fontMeta
        }
    }
}
