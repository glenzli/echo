//! Album navigation row. Its transient action menu remains with the row that
//! owns placement and interaction.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: row

    required property string label
    required property int count
    property bool selected: false
    property bool suggested: false
    property string subtitle: ""

    signal activated()
    signal saveRequested()
    signal renameRequested()
    signal deleteRequested()

    implicitHeight: subtitle.length > 0 ? 46 : 36
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

    TapHandler {
        acceptedButtons: Qt.LeftButton
        onTapped: row.activated()
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 4
        spacing: 7

        Text {
            text: row.suggested ? "✦" : "▱"
            color: row.suggested ? Theme.accent : Theme.textSecondary
            font.pixelSize: 14
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

        Button {
            id: actionButton

            Layout.preferredWidth: 27
            Layout.preferredHeight: 27
            padding: 0
            focusPolicy: Qt.NoFocus
            onClicked: {
                if (row.suggested) row.saveRequested()
                else actionMenu.popup(actionButton, 0, actionButton.height)
            }

            ToolTip.visible: hovered
            ToolTip.text: row.suggested ? qsTr("Save as album") : qsTr("Album actions")
            ToolTip.delay: 500

            background: Rectangle {
                radius: Theme.compactControlRadius
                color: actionButton.hovered ? Theme.buttonGhostHover : Theme.transparent
            }

            contentItem: Text {
                text: row.suggested ? "+" : "···"
                color: Theme.textSecondary
                font.pixelSize: row.suggested ? 17 : 13
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }

    Menu {
        id: actionMenu

        MenuItem {
            text: qsTr("Rename")
            onTriggered: row.renameRequested()
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Delete album")
            onTriggered: row.deleteRequested()
        }
    }
}
