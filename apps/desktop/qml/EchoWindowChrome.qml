//! Shared native titlebar geometry, safe areas, surface and window movement.
import QtQuick
import QtQuick.Controls
import QtQuick.Window

ToolBar {
    id: chrome
    required property var hostWindow
    objectName: "titleToolBar"
    implicitHeight: 44
    topPadding: 0
    bottomPadding: 0
    leftPadding: Math.max(SafeArea.margins.left,
        Qt.platform.os === "osx" && hostWindow.visibility !== Window.FullScreen ? 96 : 16)
    rightPadding: Math.max(SafeArea.margins.right, Qt.platform.os === "windows" ? 152 : 16)
    readonly property real windowCenterOffset: (rightPadding - leftPadding) / 2
    background: Rectangle {
        color: Theme.chrome
        DragHandler {
            target: null
            acceptedButtons: Qt.LeftButton
            onActiveChanged: if (active) chrome.hostWindow.startSystemMove()
        }
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }
    }
}
