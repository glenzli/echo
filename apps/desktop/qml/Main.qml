//! Echo application shell. Workspace chrome, page routing, and process-level
//! lifecycle live here; Audio Space and Audio Library own their presentation
//! and interaction internally.

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
    property int workspaceIndex: 0

    readonly property bool jobsActive: jobSnapshot.pending > 0 || jobSnapshot.running > 0

    function openSettings() : void {
        settingsDialog.open()
    }

    function openLibrary() : void {
        workspaceIndex = 1
        audioLibrary.refresh()
    }

    function showAudioSpace() : void {
        workspaceIndex = 0
    }

    EchoSettingsDialog {
        id: settingsDialog
    }

    header: MainTitleBar {
        hostWindow: window
        workspaceIndex: window.workspaceIndex
        searchText: audioSpace.searchText
        jobsActive: window.jobsActive
        activeJobCount: window.jobSnapshot.pending + window.jobSnapshot.running
        onSoundWallRequested: window.showAudioSpace()
        onSettingsRequested: window.openSettings()
        onSearchRequested: text => audioSpace.setSearchText(text)
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: window.workspaceIndex

        AudioSpaceWorkspace {
            id: audioSpace

            Layout.fillWidth: true
            Layout.fillHeight: true
            jobStats: window.jobSnapshot
            onOpenLibraryRequested: window.openLibrary()
        }

        AudioLibraryWorkspace {
            id: audioLibrary

            Layout.fillWidth: true
            Layout.fillHeight: true
            jobStats: window.jobSnapshot
            onCloseRequested: window.showAudioSpace()
        }
    }

    Component.onCompleted: {
        settingsDialog.uiPrefs = uiPrefs
        settingsDialog.inferencePrefs = inferencePrefs
        Theme.mode = uiPrefs.mode
        uiPrefs.modeChanged.connect(() => {
            Theme.mode = uiPrefs.mode
        })
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
