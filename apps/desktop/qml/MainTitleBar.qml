//! Echo's one window-chrome owner. Brand, Sound Wall navigation, browsing
//! controls, settings, native safe areas, and the system-move gesture share
//! one toolbar, mirroring Shadow's MainTitleBar boundary.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

ToolBar {
    id: titleBar

    required property var hostWindow
    required property int workspaceIndex
    required property string collectionLabel
    required property int soundCount
    required property string searchText
    required property string sortMode
    required property real cardSize
    required property bool selectedSoundAvailable
    required property bool jobsActive
    required property int activeJobCount

    signal soundWallRequested()
    signal settingsRequested()
    signal searchRequested(string text)
    signal sortRequested(string mode)
    signal cardSizeRequested(real size)
    signal expandRequested()

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

        RowLayout {
            anchors.left: brandRow.right
            anchors.leftMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(390, parent.width * 0.36)
            spacing: 10
            visible: titleBar.workspaceIndex === 0

            Column {
                Layout.preferredWidth: 92
                spacing: 0

                Text {
                    width: parent.width
                    text: titleBar.collectionLabel
                    color: Theme.textPrimary
                    font.pixelSize: 11
                    font.bold: true
                    elide: Text.ElideRight
                }

                Text {
                    width: parent.width
                    text: qsTr("%1 sounds").arg(titleBar.soundCount)
                    color: Theme.textDisabled
                    font.pixelSize: 9
                    elide: Text.ElideRight
                }
            }

            EchoTextField {
                Layout.fillWidth: true
                Layout.maximumWidth: 240
                implicitHeight: Theme.compactControlHeight
                text: titleBar.searchText
                placeholderText: qsTr("Search text, events, or filenames…")
                onTextEdited: titleBar.searchRequested(text)
            }
        }

        EchoIconButton {
            id: soundWallButton

            anchors.horizontalCenter: parent.horizontalCenter
            anchors.verticalCenter: parent.verticalCenter
            buttonSize: 38
            iconSize: 18
            source: "qrc:/EchoDesktop/icons/waveform.svg"
            toolTipText: qsTr("Sound Wall")
            selected: titleBar.workspaceIndex === 0
            onClicked: titleBar.soundWallRequested()
        }

        Rectangle {
            anchors.horizontalCenter: soundWallButton.horizontalCenter
            anchors.bottom: parent.bottom
            width: 28
            height: 2
            radius: 1
            visible: titleBar.workspaceIndex === 0
            color: Theme.accent
        }

        RowLayout {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 7

            ComboBox {
                id: sortBox

                visible: titleBar.workspaceIndex === 0
                implicitWidth: 96
                implicitHeight: Theme.compactControlHeight
                model: [qsTr("Date"), qsTr("Duration"), qsTr("Rating")]
                currentIndex: Math.max(0, ["date", "duration", "rating"]
                    .indexOf(titleBar.sortMode))
                onActivated: index => titleBar.sortRequested(
                    ["date", "duration", "rating"][index])

                contentItem: Text {
                    leftPadding: 10
                    rightPadding: 24
                    text: sortBox.displayText
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMeta
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }

                indicator: Text {
                    x: sortBox.width - width - 9
                    y: Math.round((sortBox.height - height) / 2) - 1
                    text: "⌄"
                    color: Theme.textSecondary
                    font.pixelSize: 13
                }

                background: Rectangle {
                    radius: Theme.controlRadius
                    color: sortBox.down ? Theme.controlPressed : Theme.control
                    border.color: sortBox.activeFocus
                        ? Theme.accent : Theme.buttonBorder
                }
            }

            Text {
                visible: titleBar.workspaceIndex === 0
                text: qsTr("Size")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Slider {
                visible: titleBar.workspaceIndex === 0
                Layout.preferredWidth: 68
                from: 230
                to: 390
                stepSize: 10
                value: titleBar.cardSize
                onMoved: titleBar.cardSizeRequested(value)
            }

            EchoButton {
                visible: titleBar.workspaceIndex === 0
                    && titleBar.selectedSoundAvailable
                text: qsTr("Expand") + " ↗"
                ghost: true
                implicitHeight: Theme.compactControlHeight
                onClicked: titleBar.expandRequested()
            }

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
