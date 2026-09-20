//! Library workspace navigation and editor actions inside EchoWindowChrome.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

EchoWindowChrome {
    id: titleBar

    required property var editor
    required property var assembly
    required property int workspaceIndex
    required property bool editorAvailable
    required property bool jobsActive
    required property int activeJobCount

    signal soundWallRequested()
    signal soundEditorRequested()
    signal soundAssemblyRequested()
    signal settingsRequested()
    signal materialsRequested()

    Accessible.name: qsTr("Echo toolbar")

    contentItem: Item {
        RowLayout {
            id: brandRow

            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            spacing: 10

            Text {
                objectName: "brandLabel"
                Layout.alignment: Qt.AlignVCenter
                text: "ECHO"
                color: Theme.textPrimary
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.letterSpacing: 2.5
            }

            EchoIconButton {
                objectName: "independentEditorButton"
                Layout.alignment: Qt.AlignVCenter
                source: "qrc:/EchoDesktop/icons/edit.svg"
                toolTipText: qsTr("Independent editing…")
                onClicked: independentEditor.launchEditor([])
            }
            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 18
                color: Theme.border
            }
        }

        Row {
            id: workspaceNavigation
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.horizontalCenterOffset: titleBar.windowCenterOffset
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            spacing: 10

            Item {
                width: 46
                height: parent.height

                EchoIconButton {
                    anchors.centerIn: parent
                    buttonSize: 30
                    iconSize: 18
                    source: "qrc:/EchoDesktop/icons/waveform.svg"
                    toolTipText: qsTr("Memory library")
                    selected: titleBar.workspaceIndex === 0
                    onClicked: titleBar.soundWallRequested()
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    width: 24
                    height: 2
                    radius: 1
                    visible: titleBar.workspaceIndex === 0
                    color: Theme.accent
                }
            }

            Item {
                width: 46
                height: parent.height

                EchoIconButton {
                    anchors.centerIn: parent
                    buttonSize: 30
                    iconSize: 18
                    source: "qrc:/EchoDesktop/icons/edit.svg"
                    toolTipText: qsTr("Edit sound")
                    enabled: titleBar.editorAvailable
                    selected: titleBar.workspaceIndex === 1
                    onClicked: titleBar.soundEditorRequested()
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    width: 24
                    height: 2
                    radius: 1
                    visible: titleBar.workspaceIndex === 1
                    color: Theme.accent
                }
            }

            Item {
                width: 46
                height: parent.height

                EchoIconButton {
                    anchors.centerIn: parent
                    buttonSize: 30
                    iconSize: 18
                    source: "qrc:/EchoDesktop/icons/assembly.svg"
                    toolTipText: qsTr("Projects")
                    selected: titleBar.workspaceIndex === 3
                    onClicked: titleBar.soundAssemblyRequested()
                }

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    width: 24
                    height: 2
                    radius: 1
                    visible: titleBar.workspaceIndex === 3
                    color: Theme.accent
                }
            }
            Item {
                width: 46
                height: parent.height
                EchoIconButton {
                    anchors.centerIn: parent
                    buttonSize: 30
                    iconSize: 18
                    source: "qrc:/EchoDesktop/icons/folder.svg"
                    toolTipText: qsTr("Materials")
                    selected: titleBar.workspaceIndex === 4
                    onClicked: titleBar.materialsRequested()
                }
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    width: 24; height: 2; radius: 1
                    visible: titleBar.workspaceIndex === 4
                    color: Theme.accent
                }
            }
        }

        RowLayout {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4

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

            Row {
                readonly property var activeEditor: titleBar.workspaceIndex === 3
                    ? titleBar.assembly : titleBar.editor
                visible: (titleBar.workspaceIndex === 1 || titleBar.workspaceIndex === 3)
                    && activeEditor !== null && activeEditor !== undefined
                spacing: 6
                Layout.rightMargin: 3

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 7
                    height: 7
                    radius: width / 2
                    color: parent.activeEditor.dirty
                        ? Theme.warningText : Theme.accentSelectionText
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: parent.activeEditor.dirty ? qsTr("Draft") : qsTr("Saved")
                    color: parent.activeEditor.dirty
                        ? Theme.warningText : Theme.accentSelectionText
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.55
                }
            }

            EchoIconButton {
                readonly property var activeEditor: titleBar.workspaceIndex === 3
                    ? titleBar.assembly : titleBar.editor
                visible: titleBar.workspaceIndex === 1 || titleBar.workspaceIndex === 3
                source: "qrc:/EchoDesktop/icons/undo.svg"
                toolTipText: qsTr("Undo")
                enabled: activeEditor !== null && activeEditor !== undefined
                    && activeEditor.canUndo
                buttonSize: 28
                iconSize: 16
                onClicked: activeEditor.undo()
            }

            EchoIconButton {
                readonly property var activeEditor: titleBar.workspaceIndex === 3
                    ? titleBar.assembly : titleBar.editor
                visible: titleBar.workspaceIndex === 1 || titleBar.workspaceIndex === 3
                source: "qrc:/EchoDesktop/icons/redo.svg"
                toolTipText: qsTr("Redo")
                enabled: activeEditor !== null && activeEditor !== undefined
                    && activeEditor.canRedo
                buttonSize: 28
                iconSize: 16
                onClicked: activeEditor.redo()
            }

            EchoButton {
                readonly property var activeEditor: titleBar.workspaceIndex === 3
                    ? titleBar.assembly : titleBar.editor
                visible: titleBar.workspaceIndex === 1 || titleBar.workspaceIndex === 3
                text: qsTr("Save version")
                enabled: activeEditor !== null && activeEditor !== undefined
                    && activeEditor.dirty
                implicitWidth: 82
                implicitHeight: 27
                onClicked: activeEditor.save()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/tune.svg"
                toolTipText: qsTr("Settings")
                buttonSize: 28
                iconSize: 16
                onClicked: titleBar.settingsRequested()
            }
        }
    }
}
