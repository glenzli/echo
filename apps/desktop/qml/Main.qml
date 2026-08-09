//! Echo application shell. Workspace chrome and process-level lifecycle live
//! here; Audio Space and asset detail own their presentation internally.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

ApplicationWindow {
    id: window

    width: 1180
    height: 760
    minimumWidth: 900
    minimumHeight: 560
    visible: true
    title: Qt.platform.os === "osx" ? "" : qsTr("Echo")
    flags: Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint
    color: Theme.window

    property int workspaceIndex: 0
    property var jobSnapshot: ({ pending: 0, running: 0, done: 0, failed: 0 })
    property bool jobsWereActive: false

    readonly property bool jobsActive: jobSnapshot.pending > 0 || jobSnapshot.running > 0

    EchoSettingsDialog {
        id: settingsDialog
    }

    Rectangle {
        id: titleBar

        anchors.left: parent.left
        anchors.right: parent.right
        height: 44
        y: window.visibility === Window.FullScreen ? 0 : -32
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
            anchors.leftMargin: Math.max(
                SafeArea.margins.left,
                Qt.platform.os === "osx" && window.visibility !== Window.FullScreen ? 96 : 16
            )
            anchors.rightMargin: Math.max(
                SafeArea.margins.right,
                Qt.platform.os === "windows" ? 152 : 16
            )
            spacing: 8

            Rectangle {
                Layout.preferredWidth: 24
                Layout.preferredHeight: 24
                radius: 7
                color: Theme.accentSurface

                EchoIcon {
                    anchors.centerIn: parent
                    source: "qrc:/EchoDesktop/icons/waveform.svg"
                    size: 15
                    color: Theme.accentSelectionText
                }
            }

            Text {
                text: qsTr("Echo")
                color: Theme.textPrimary
                font.pixelSize: 14
                font.bold: true
            }

            Text {
                text: "·"
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
            }

            Text {
                text: window.workspaceIndex === 0 ? qsTr("Audio Space") : qsTr("Library")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
            }

            Item {
                Layout.fillWidth: true

                DragHandler {
                    acceptedButtons: Qt.LeftButton
                    target: null
                    onActiveChanged: {
                        if (active) {
                            window.startSystemMove()
                        }
                    }
                }
            }

            Rectangle {
                visible: window.jobsActive
                Layout.preferredWidth: jobLabel.implicitWidth + 18
                Layout.preferredHeight: 24
                radius: 12
                color: Theme.surfaceSubtle

                Text {
                    id: jobLabel
                    anchors.centerIn: parent
                    text: qsTr("Indexing %1").arg(
                        window.jobSnapshot.pending + window.jobSnapshot.running)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                toolTipText: qsTr("Audio Space")
                selected: window.workspaceIndex === 0
                checkable: true
                checked: window.workspaceIndex === 0
                onClicked: window.workspaceIndex = 0
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/folder.svg"
                toolTipText: qsTr("Library")
                selected: window.workspaceIndex === 1
                checkable: true
                checked: window.workspaceIndex === 1
                onClicked: window.workspaceIndex = 1
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/tune.svg"
                toolTipText: qsTr("Settings")
                onClicked: settingsDialog.open()
            }
        }
    }

    Item {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: titleBar.bottom
        anchors.bottom: parent.bottom

        AudioSpaceWorkspace {
            id: audioSpace

            anchors.fill: parent
            anchors.margins: 20
            visible: window.workspaceIndex === 0
            jobStats: window.jobSnapshot
            onOpenLibraryRequested: window.workspaceIndex = 1
        }

        LibraryPanel {
            anchors.fill: parent
            anchors.margins: 24
            visible: window.workspaceIndex === 1
        }
    }

    Component.onCompleted: {
        settingsDialog.uiPrefs = uiPrefs
        settingsDialog.modelPrefs = modelPrefs
        Theme.mode = uiPrefs.mode
        uiPrefs.modeChanged.connect(() => {
            Theme.mode = uiPrefs.mode
        })
        backend.startWorkers(modelPrefs.modelRoot, modelPrefs.python,
                             modelPrefs.workerScript)
        backend.queueScans()
        audioSpace.refreshAssets()
        jobTimer.start()
    }

    Timer {
        id: jobTimer

        interval: 750
        repeat: true
        onTriggered: {
            const snapshot = backend.jobStats()
            const active = snapshot.pending > 0 || snapshot.running > 0
            window.jobSnapshot = snapshot
            if (window.jobsWereActive && !active) {
                backend.refresh()
            }
            window.jobsWereActive = active
        }
    }
}
