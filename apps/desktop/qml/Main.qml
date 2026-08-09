//! Echo application shell. Workspace chrome and process-level lifecycle live
//! here; Audio Space, playback, and inspection own their presentation
//! internally. Library management is a transient toolbar surface.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

ApplicationWindow {
    id: window

    width: 1500
    height: 900
    minimumWidth: 1240
    minimumHeight: 720
    visible: true
    title: Qt.platform.os === "osx" ? "" : qsTr("Echo")
    flags: Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint
    color: Theme.window

    property var jobSnapshot: ({ pending: 0, running: 0, done: 0, failed: 0 })
    property bool jobsWereActive: false

    readonly property bool jobsActive: jobSnapshot.pending > 0 || jobSnapshot.running > 0

    EchoSettingsDialog {
        id: settingsDialog
    }

    LibraryDialog {
        id: libraryDialog
    }

    Rectangle {
        id: titleBar

        anchors.left: parent.left
        anchors.right: parent.right
        height: 48
        y: 0
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
                text: qsTr("Audio Space")
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
                selected: true
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/folder.svg"
                toolTipText: qsTr("Manage library")
                onClicked: libraryDialog.open()
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
            anchors.margins: 14
            jobStats: window.jobSnapshot
            onOpenLibraryRequested: libraryDialog.open()
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
