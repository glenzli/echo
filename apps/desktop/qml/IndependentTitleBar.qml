//! Document identity, editor selection and actions inside the shared window chrome.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

EchoWindowChrome {
    id: titleBar
    required property string projectName
    required property bool dirty
    required property bool processing
    required property bool hasSource
    required property bool multitrack
    signal waveformRequested()
    signal multitrackRequested()
    signal openRequested()
    signal saveRequested()
    signal exportRequested()
    Accessible.name: qsTr("Echo · Independent editing")

    contentItem: Item {
        RowLayout {
            id: identity
            objectName: "projectIdentity"
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(320, modes.x - 16)
            spacing: 12
            Text {
                text: "ECHO"
                color: Theme.textPrimary
                font.pixelSize: 14
                font.weight: Font.DemiBold
                font.letterSpacing: 2.5
                Layout.alignment: Qt.AlignVCenter
            }
            Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 18; color: Theme.border }
            Text {
                objectName: "projectTitle"
                Layout.fillWidth: true
                text: titleBar.projectName
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
                elide: Text.ElideMiddle
                maximumLineCount: 1
                ToolTip.visible: titleHover.hovered
                ToolTip.text: titleBar.projectName
                ToolTip.delay: 600
                HoverHandler { id: titleHover }
            }
            Rectangle {
                visible: titleBar.dirty
                Layout.preferredWidth: 6; Layout.preferredHeight: 6
                radius: 3; color: Theme.warningText
                Accessible.name: qsTr("Unsaved changes")
            }
        }
        EchoSegmentedControl {
            id: modes
            objectName: "editorModes"
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: titleBar.windowCenterOffset
            width: 250
            model: [qsTr("Waveform / Spectrum"), qsTr("Multitrack")]
            currentIndex: titleBar.multitrack ? 1 : 0
            enabled: titleBar.hasSource && !titleBar.processing
            onActivated: index => { if (index === 0) titleBar.waveformRequested(); else titleBar.multitrackRequested(); }
        }
        RowLayout {
            id: actions
            objectName: "projectActions"
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6
            EchoIconButton {
                objectName: "openAudioButton"
                source: "qrc:/EchoDesktop/icons/folder.svg"
                toolTipText: qsTr("Open audio…")
                enabled: !titleBar.processing
                onClicked: titleBar.openRequested()
            }
            EchoIconButton {
                objectName: "saveProjectButton"
                source: "qrc:/EchoDesktop/icons/recipe-save.svg"
                toolTipText: qsTr("Save project")
                enabled: !titleBar.processing
                onClicked: titleBar.saveRequested()
            }
            Rectangle { Layout.preferredWidth: 1; Layout.preferredHeight: 18; color: Theme.border }
            EchoButton {
                objectName: "exportAudioButton"
                implicitHeight: Theme.compactControlHeight
                text: qsTr("Export audio…")
                enabled: titleBar.hasSource && !titleBar.processing
                onClicked: titleBar.exportRequested()
            }
        }
    }
}
