//! Bounded, text-first choice control for effect modes and filter shapes.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: control

    property var model: []
    property int currentIndex: 0
    signal activated(int index)

    implicitWidth: Math.max(180, choices.implicitWidth + 6)
    implicitHeight: 30
    radius: Theme.controlRadius
    color: Theme.panelInset
    border.width: 1
    border.color: Theme.border

    RowLayout {
        id: choices
        anchors.fill: parent
        anchors.margins: 3
        spacing: 2

        Repeater {
            model: control.model

            delegate: Button {
                id: choice

                required property int index
                required property var modelData

                Layout.fillWidth: true
                Layout.fillHeight: true
                padding: 0
                focusPolicy: Qt.NoFocus
                onClicked: control.activated(choice.index)

                background: Rectangle {
                    radius: Theme.compactControlRadius - 1
                    color: choice.index === control.currentIndex ? Theme.panelRaised : choice.hovered ? Theme.buttonGhostHover : Theme.transparent
                    border.width: choice.index === control.currentIndex ? 1 : 0
                    border.color: Theme.borderStrong
                }

                contentItem: Text {
                    text: String(choice.modelData)
                    color: choice.index === control.currentIndex ? Theme.textPrimary : Theme.textMuted
                    font.pixelSize: Theme.fontMeta
                    font.weight: choice.index === control.currentIndex ? Font.DemiBold : Font.Normal
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }
        }
    }
}
