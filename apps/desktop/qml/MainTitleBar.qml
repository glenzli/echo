//! Echo's one window-chrome owner. Brand, primary workspace navigation,
//! settings, native safe areas, and the system-move gesture share one toolbar,
//! mirroring Shadow's MainTitleBar boundary.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

ToolBar {
    id: titleBar

    required property var hostWindow
    required property int workspaceIndex
    required property bool editorAvailable
    required property bool jobsActive
    required property int activeJobCount

    signal soundWallRequested()
    signal soundEditorRequested()
    signal settingsRequested()

    objectName: "titleToolBar"
    Accessible.name: qsTr("Echo toolbar")
    implicitHeight: 48
    topPadding: 0
    bottomPadding: 0
    leftPadding: Math.max(
        SafeArea.margins.left,
        Qt.platform.os === "osx"
            && hostWindow.visibility !== Window.FullScreen ? 96 : 16
    )
    rightPadding: Math.max(
        SafeArea.margins.right,
        Qt.platform.os === "windows" ? 152 : 16
    )

    background: Rectangle {
        color: Theme.chrome

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }
    }

    contentItem: Item {
        Item {
            anchors.fill: parent

            DragHandler {
                target: null
                acceptedButtons: Qt.LeftButton
                onActiveChanged: {
                    if (active) {
                        titleBar.hostWindow.startSystemMove()
                    }
                }
            }
        }

        Row {
            id: brandRow

            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            spacing: 10

            Text {
                text: "ECHO"
                color: Theme.textPrimary
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.letterSpacing: 2.5
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 1
                height: 18
                color: Theme.border
            }
        }

        Row {
            id: workspaceNavigation
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            spacing: 6

            Item {
                width: 44
                height: parent.height

                EchoIconButton {
                    anchors.centerIn: parent
                    buttonSize: 38
                    iconSize: 18
                    source: "qrc:/EchoDesktop/icons/waveform.svg"
                    toolTipText: qsTr("Audio Space")
                    selected: titleBar.workspaceIndex === 0
                    onClicked: titleBar.soundWallRequested()
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    width: 28
                    height: 2
                    radius: 1
                    visible: titleBar.workspaceIndex === 0
                    color: Theme.accent
                }
            }

            Item {
                width: 44
                height: parent.height

                EchoIconButton {
                    anchors.centerIn: parent
                    buttonSize: 38
                    iconSize: 18
                    source: "qrc:/EchoDesktop/icons/edit.svg"
                    toolTipText: qsTr("Sound Adjustments")
                    enabled: titleBar.editorAvailable
                    selected: titleBar.workspaceIndex === 1
                    onClicked: titleBar.soundEditorRequested()
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    width: 28
                    height: 2
                    radius: 1
                    visible: titleBar.workspaceIndex === 1
                    color: Theme.accent
                }
            }
        }

        RowLayout {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 7

            Rectangle {
                visible: titleBar.jobsActive
                Layout.preferredWidth: jobLabel.implicitWidth + 18
                Layout.preferredHeight: 24
                radius: 12
                color: Theme.surfaceSubtle

                Text {
                    id: jobLabel
                    anchors.centerIn: parent
                    text: qsTr("Indexing %1").arg(titleBar.activeJobCount)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/tune.svg"
                toolTipText: qsTr("Settings")
                onClicked: titleBar.settingsRequested()
            }
        }
    }
}
