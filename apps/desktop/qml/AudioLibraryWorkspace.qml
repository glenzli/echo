//! Audio Library workspace. Owns source-folder admission, root projection,
//! missing-file presentation, and the native folder chooser interaction.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import EchoDesktop

Item {
    id: workspace

    property var jobStats: ({ pending: 0, running: 0, done: 0, failed: 0 })
    property string actionMessage: ""
    property bool actionFailed: false

    signal closeRequested()

    readonly property bool jobsActive: jobStats.pending > 0 || jobStats.running > 0

    function refresh() : void {
        rootModel.refresh()
        missingModel.refresh()
    }

    function addFolder(folder: url) : void {
        if (backend.addRoot(folder)) {
            actionFailed = false
            actionMessage = qsTr("Folder added. Echo is preparing its recordings.")
            refresh()
        } else {
            actionFailed = true
            actionMessage = qsTr("Echo could not add that folder.")
        }
    }

    FolderDialog {
        id: folderDialog

        title: qsTr("Choose an audio folder")
        onAccepted: workspace.addFolder(selectedFolder)
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.window
    }

    ScrollView {
        id: scrollView

        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

        ColumnLayout {
            width: Math.min(840, Math.max(640, scrollView.availableWidth - 64))
            x: Math.round((scrollView.availableWidth - width) / 2)
            spacing: 18

            Item { Layout.preferredHeight: 28 }

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Rectangle {
                    Layout.preferredWidth: 40
                    Layout.preferredHeight: 40
                    radius: 9
                    color: Theme.accentSurface

                    EchoIcon {
                        anchors.centerIn: parent
                        source: "qrc:/EchoDesktop/icons/folder.svg"
                        color: Theme.accentSelectionText
                        size: 20
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Audio Library")
                        color: Theme.textPrimary
                        font.pixelSize: 20
                        font.bold: true
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Manage watched folders while original recordings remain untouched.")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }

                Rectangle {
                    visible: workspace.jobsActive
                    Layout.preferredWidth: activityText.implicitWidth + 18
                    Layout.preferredHeight: 24
                    radius: 12
                    color: Theme.accentSurfaceQuiet

                    Text {
                        id: activityText
                        anchors.centerIn: parent
                        text: qsTr("Preparing %1").arg(
                            workspace.jobStats.pending + workspace.jobStats.running)
                        color: Theme.accentSelectionText
                        font.pixelSize: Theme.fontMeta
                    }
                }

                EchoButton {
                    text: qsTr("Back to Audio Space")
                    ghost: true
                    onClicked: workspace.closeRequested()
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: qsTr("WATCHED FOLDERS")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 0.5
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: qsTr("%1 folders").arg(rootModel.count)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: addFolderContent.implicitHeight + 30
                radius: 9
                color: Theme.panelRaised
                border.color: Theme.border

                RowLayout {
                    id: addFolderContent

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 16
                    anchors.rightMargin: 14
                    spacing: 12

                    Rectangle {
                        Layout.preferredWidth: 36
                        Layout.preferredHeight: 36
                        radius: Theme.controlRadius
                        color: Theme.surfaceSubtle

                        EchoIcon {
                            anchors.centerIn: parent
                            source: "qrc:/EchoDesktop/icons/folder.svg"
                            color: Theme.textSecondary
                            size: 18
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("ADD AUDIO FOLDER")
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontSection
                            font.bold: true
                        }

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Choose a folder; Echo will watch it and prepare new or changed recordings in the background.")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }

                    EchoButton {
                        text: qsTr("Choose Folder…")
                        enabled: !workspace.jobsActive
                        onClicked: folderDialog.open()
                    }
                }
            }

            Rectangle {
                visible: workspace.actionMessage.length > 0
                Layout.fillWidth: true
                Layout.preferredHeight: 38
                radius: Theme.controlRadius
                color: workspace.actionFailed ? Theme.warningSurface : Theme.accentSurfaceQuiet

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    text: workspace.actionMessage
                    color: workspace.actionFailed ? Theme.warningText : Theme.accentSelectionText
                    font.pixelSize: Theme.fontMeta
                    verticalAlignment: Text.AlignVCenter
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                Repeater {
                    model: rootModel

                    delegate: Rectangle {
                        required property var modelData

                        Layout.fillWidth: true
                        Layout.preferredHeight: 58
                        radius: 8
                        color: Theme.panel
                        border.color: Theme.border

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 10
                            spacing: 10

                            EchoIcon {
                                source: "qrc:/EchoDesktop/icons/folder.svg"
                                color: Theme.textSecondary
                                size: 18
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.root
                                    color: Theme.textPrimary
                                    font.pixelSize: Theme.fontBody
                                    elide: Text.ElideMiddle
                                }

                                Text {
                                    text: qsTr("Watched folder")
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                }
                            }

                            EchoIconButton {
                                source: "qrc:/EchoDesktop/icons/stop.svg"
                                toolTipText: qsTr("Remove folder")
                                buttonSize: 28
                                iconSize: 14
                                onClicked: {
                                    backend.removeRoot(modelData.id)
                                    rootModel.refresh()
                                    workspace.actionFailed = false
                                    workspace.actionMessage = qsTr("Folder removed. Imported information is preserved.")
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    visible: rootModel.count === 0
                    Layout.fillWidth: true
                    Layout.preferredHeight: 58
                    radius: 8
                    color: Theme.panel
                    border.color: Theme.border

                    Text {
                        anchors.centerIn: parent
                        text: qsTr("No watched folders yet")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontBody
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: qsTr("MISSING RECORDINGS")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 0.5
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: qsTr("%1 missing").arg(missingModel.count)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: missingModel.count > 0
                    ? Math.min(156, missingModel.count * 38 + 16) : 56
                radius: 8
                color: Theme.panelRaised
                border.color: Theme.border

                ListView {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 4
                    clip: true
                    model: missingModel

                    delegate: RowLayout {
                        required property var modelData

                        width: ListView.view.width
                        height: 34
                        spacing: 8

                        Rectangle {
                            Layout.preferredWidth: 54
                            Layout.preferredHeight: 18
                            radius: 9
                            color: Theme.warningSurface

                            Text {
                                anchors.centerIn: parent
                                text: qsTr("Missing")
                                color: Theme.warningText
                                font.pixelSize: Theme.fontMeta
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: modelData.path
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                            elide: Text.ElideMiddle
                        }
                    }
                }

                Text {
                    visible: missingModel.count === 0
                    anchors.centerIn: parent
                    text: qsTr("All original recordings are available")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontBody
                }
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Echo keeps analysis results while a recording is offline and reconnects them when the original returns.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: qsTr("STORAGE")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 0.5
                }

                Item { Layout.fillWidth: true }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 58
                radius: 8
                color: Theme.panelRaised
                border.color: Theme.border

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    spacing: 10

                    Text {
                        text: qsTr("Rebuildable cache")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontBody
                    }

                    Text {
                        Layout.fillWidth: true
                        text: backend.cacheRoot
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        elide: Text.ElideMiddle
                    }
                }
            }

            Item { Layout.preferredHeight: 28 }
        }
    }

    ListModel {
        id: rootModel

        function refresh() : void {
            clear()
            const roots = backend.listRoots()
            for (let index = 0; index < roots.length; ++index) {
                append(roots[index])
            }
        }
    }

    ListModel {
        id: missingModel

        function refresh() : void {
            clear()
            const assets = backend.listAssets()
            for (let index = 0; index < assets.length; ++index) {
                if (assets[index].pathStatus === "missing") {
                    append(assets[index])
                }
            }
        }
    }

    Connections {
        target: backend

        function onAssetsChanged() : void {
            workspace.refresh()
        }
    }

    Component.onCompleted: refresh()
}
