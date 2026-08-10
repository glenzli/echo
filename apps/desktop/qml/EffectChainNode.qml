//! Compact direct-manipulation row for one authored singleton effect node.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: node

    required property int kind
    required property int orderIndex
    required property string title
    required property string summary
    required property string iconSource
    required property bool nodeEnabled
    required property bool selected
    required property bool canMoveUp
    required property bool canMoveDown
    required property bool terminal

    signal selectedRequested()
    signal enabledRequested(bool enabled)
    signal moveRequested(int direction)

    implicitWidth: 190
    implicitHeight: 54
    radius: Theme.compactControlRadius
    color: selected ? Theme.surfaceSelected : nodeHover.hovered
        ? Theme.surfaceSubtle : "transparent"
    border.width: selected ? 1 : 0
    border.color: selected ? Theme.accent : "transparent"

    Rectangle {
        x: 16
        y: node.height / 2
        width: 1
        height: node.terminal ? 0 : node.height + 5
        color: Theme.borderStrong
    }

    Rectangle {
        x: 12
        y: Math.round((parent.height - height) / 2)
        width: 9
        height: 9
        radius: 5
        color: node.nodeEnabled ? Theme.accent : Theme.panelRaised
        border.width: 1
        border.color: node.nodeEnabled ? Theme.accent : Theme.textDisabled
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 28
        anchors.rightMargin: 7
        spacing: 7

        EchoIcon {
            source: node.iconSource
            size: 16
            color: node.nodeEnabled ? Theme.textPrimary : Theme.textDisabled
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Text {
                Layout.fillWidth: true
                text: node.title
                color: node.nodeEnabled ? Theme.textPrimary : Theme.textSecondary
                font.pixelSize: Theme.fontBody
                font.weight: node.selected ? Font.DemiBold : Font.Normal
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                text: node.nodeEnabled ? node.summary : qsTr("Bypassed")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        ColumnLayout {
            visible: node.selected && !node.terminal
            spacing: -2

            Text {
                text: "↑"
                color: node.canMoveUp ? Theme.textSecondary : Theme.textDisabled
                font.pixelSize: 13
                horizontalAlignment: Text.AlignHCenter
                Layout.preferredWidth: 17
                TapHandler {
                    enabled: node.canMoveUp
                    onTapped: node.moveRequested(-1)
                }
            }
            Text {
                text: "↓"
                color: node.canMoveDown ? Theme.textSecondary : Theme.textDisabled
                font.pixelSize: 13
                horizontalAlignment: Text.AlignHCenter
                Layout.preferredWidth: 17
                TapHandler {
                    enabled: node.canMoveDown
                    onTapped: node.moveRequested(1)
                }
            }
        }

        Rectangle {
            implicitWidth: 30
            implicitHeight: 17
            radius: height / 2
            color: node.nodeEnabled ? Theme.accent : Theme.track

            Rectangle {
                width: 13
                height: 13
                radius: 7
                y: 2
                x: node.nodeEnabled ? parent.width - width - 2 : 2
                color: node.nodeEnabled ? Theme.accentText : Theme.panelRaised
                Behavior on x { NumberAnimation { duration: 90 } }
            }

            TapHandler {
                onTapped: node.enabledRequested(!node.nodeEnabled)
            }
        }
    }

    HoverHandler { id: nodeHover }
    TapHandler {
        acceptedButtons: Qt.LeftButton
        gesturePolicy: TapHandler.ReleaseWithinBounds
        onTapped: node.selectedRequested()
    }
}
