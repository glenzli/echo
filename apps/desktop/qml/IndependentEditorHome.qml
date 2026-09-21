//! Standalone start page: one action row and a bounded, direct recent-project list.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: home
    required property var controller
    property bool narrationAvailable: true
    readonly property var projects: controller.recentProjects || []
    signal openAudioRequested()
    signal openProjectRequested()
    signal narrationRequested()

    ColumnLayout {
        id: content
        anchors.centerIn: parent
        width: Math.min(820, home.width - 80)
        height: Math.min(590, home.height - 64)
        spacing: 0
        Text { text: qsTr("Sound editing"); color: Theme.textPrimary; font.pixelSize: 28; font.weight: Font.DemiBold }
        Text { Layout.topMargin: 8; text: qsTr("Drop audio here, or pick up where you left off."); color: Theme.textMuted; font.pixelSize: Theme.fontBody }
        RowLayout {
            id: actions
            objectName: "editorStartActions"
            Layout.fillWidth: true; Layout.topMargin: 26; Layout.bottomMargin: 34
            spacing: 12
            Repeater {
                model: [
                    {key: "audio", title: qsTr("Open audio…"), detail: qsTr("Edit, repair or arrange"), icon: "waveform"},
                    {key: "project", title: qsTr("Open project…"), detail: qsTr("Continue an Echo project"), icon: "folder"},
                    {key: "narration", title: qsTr("Generate narration…"), detail: qsTr("Create a voice from text"), icon: "sparkles"}
                ]
                delegate: AbstractButton {
                    id: action
                    required property var modelData
                    objectName: "editorStart-" + modelData.key
                    Layout.fillWidth: true; Layout.preferredWidth: 1
                    implicitHeight: 82
                    enabled: modelData.key !== "narration" || home.narrationAvailable
                    Accessible.name: modelData.title
                    Accessible.role: Accessible.Button
                    onClicked: {
                        if (modelData.key === "audio") home.openAudioRequested();
                        else if (modelData.key === "project") home.openProjectRequested();
                        else home.narrationRequested();
                    }
                    background: Rectangle {
                        color: action.down ? Theme.accentSurface : action.hovered ? Theme.accentSurfaceQuiet : Theme.panelRaised
                        radius: Theme.panelRadius
                        border.color: action.activeFocus ? Theme.focusRing : action.hovered ? Theme.borderStrong : Theme.border
                    }
                    contentItem: RowLayout {
                        anchors.fill: parent; anchors.margins: 16; spacing: 14
                        EchoIcon { source: "qrc:/EchoDesktop/icons/" + action.modelData.icon + ".svg"; size: 22; color: action.enabled ? Theme.accent : Theme.textDisabled }
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 6
                            Text { Layout.fillWidth: true; text: action.modelData.title; elide: Text.ElideRight; color: action.enabled ? Theme.textPrimary : Theme.textDisabled; font.pixelSize: 13; font.weight: Font.Medium }
                            Text { Layout.fillWidth: true; text: action.modelData.detail; elide: Text.ElideRight; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                        }
                    }
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.bottomMargin: 12
            Text { text: qsTr("Recent projects"); color: Theme.textPrimary; font.pixelSize: 14; font.weight: Font.DemiBold }
            Item { Layout.fillWidth: true }
            Text { visible: home.projects.length > 0; text: home.projects.length; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
        }
        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }
        ListView {
            id: recentList
            objectName: "editorRecentProjects"
            Layout.fillWidth: true; Layout.fillHeight: true
            clip: true; boundsBehavior: Flickable.StopAtBounds
            model: home.projects
            ScrollBar.vertical: ScrollBar {}
            delegate: AbstractButton {
                id: row
                required property var modelData
                required property int index
                readonly property bool recovery: modelData.kind === "recovery"
                readonly property string dateText: Qt.formatDateTime(new Date(Number(modelData.modifiedMillis)), "yyyy/MM/dd hh:mm")
                objectName: "editorRecentProject-" + index
                width: recentList.width; height: 66
                Accessible.name: modelData.title + " · " + (recovery ? qsTr("Recoverable edit") : qsTr("Saved project")) + " · " + dateText
                Accessible.role: Accessible.Button
                onClicked: home.controller.openRecentProject(modelData.path, recovery)
                background: Rectangle {
                    color: row.down ? Theme.accentSurface : row.hovered || row.activeFocus ? Theme.accentSurfaceQuiet : Theme.transparent
                    radius: Theme.controlRadius
                    border.width: row.activeFocus ? 1 : 0; border.color: Theme.focusRing
                }
                contentItem: RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 14; spacing: 14
                    EchoIcon { source: row.recovery ? "qrc:/EchoDesktop/icons/history.svg" : "qrc:/EchoDesktop/icons/assembly.svg"; size: 20; color: Theme.textMuted }
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 5
                        Text { Layout.fillWidth: true; text: row.modelData.title; textFormat: Text.PlainText; elide: Text.ElideMiddle; color: Theme.textPrimary; font.pixelSize: 13 }
                        Text { Layout.fillWidth: true; text: row.recovery ? qsTr("%1 sources").arg(row.modelData.sourceCount) : row.modelData.folder; textFormat: Text.PlainText; elide: Text.ElideMiddle; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                    }
                    Text { Layout.preferredWidth: 88; horizontalAlignment: Text.AlignRight; text: row.recovery ? qsTr("Recoverable edit") : qsTr("Saved project"); color: row.recovery ? Theme.accent : Theme.textMuted; font.pixelSize: Theme.fontMeta }
                    Text { Layout.preferredWidth: 122; horizontalAlignment: Text.AlignRight; text: row.dateText; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
                }
            }
            ColumnLayout {
                anchors.centerIn: parent; spacing: 10; visible: !home.projects.length
                Text { Layout.alignment: Qt.AlignHCenter; text: qsTr("No recent projects yet"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                Text { Layout.alignment: Qt.AlignHCenter; text: qsTr("Saved projects and unfinished edits will appear here."); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            }
        }
    }
}
