//! Complete selected-sound workspace reached from the Sound Wall. Navigation
//! stays outside AudioPlaybackWorkspace so playback remains one cohesive owner.

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: expanded

    required property var asset
    required property var jobStats

    signal closeRequested()

    color: Theme.window

    function playFrom(millis: int) : void {
        playback.playFrom(millis)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 46
            color: Theme.chrome

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: Theme.border
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 15
                spacing: 8

                EchoButton {
                    text: "‹ " + qsTr("Sound Wall")
                    ghost: true
                    implicitHeight: 30
                    onClicked: expanded.closeRequested()
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: qsTr("Expanded sound")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 0.8
                }
            }
        }

        AudioPlaybackWorkspace {
            id: playback
            Layout.fillWidth: true
            Layout.fillHeight: true
            asset: expanded.asset
            jobStats: expanded.jobStats
        }
    }
}
