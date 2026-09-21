//! Compact timeline annotation lane; audio and source references remain in track lanes.
import QtQuick
import QtQuick.Controls

Rectangle {
    id: lane
    required property var workspace
    implicitHeight: 28
    color: Theme.panelInset
    clip: true
    Text {
        x: 12; anchors.verticalCenter: parent.verticalCenter
        text: qsTr("MARKERS"); color: Theme.textMuted; font.pixelSize: 9; font.letterSpacing: 1
    }
    Item {
        x: lane.workspace.trackHeaderWidth
        width: parent.width - x
        height: parent.height
        clip: true
        Repeater {
            model: lane.workspace.markers
            delegate: Rectangle {
                required property var modelData
                readonly property bool range: modelData.endMillis !== undefined
                x: modelData.startMillis * lane.workspace.pixelsPerSecond / 1000 - lane.workspace.scrollPosition
                y: range ? 10 : 0
                width: range ? Math.max(8, (modelData.endMillis - modelData.startMillis) * lane.workspace.pixelsPerSecond / 1000) : 8
                height: range ? parent.height - 12 : parent.height
                color: range ? Theme.accentSurface : "transparent"
                border.color: range ? Theme.accentBorder : "transparent"
                radius: 2
                Rectangle { visible: parent.range; width: 2; height: parent.height; color: Theme.accent }
                Rectangle { visible: !parent.range; x: 1; y: 2; width: 6; height: 6; rotation: 45; color: Theme.accent }
                Text {
                    x: 5; anchors.verticalCenter: parent.verticalCenter
                    width: Math.max(0, Math.min(120, parent.width - 10))
                    visible: parent.range && parent.width > 30
                    text: modelData.name; color: Theme.accentSelectionText; font.pixelSize: 9; elide: Text.ElideRight
                }
                HoverHandler { id: hover }
                TapHandler {
                    onTapped: { lane.workspace.selectedMarkerId = modelData.id; lane.workspace.markersVisible = true; lane.workspace.seekMarker(modelData.id); }
                }
                ToolTip.visible: hover.hovered
                ToolTip.text: modelData.name
                Accessible.role: Accessible.Button
                Accessible.name: modelData.name
                Accessible.onPressAction: { lane.workspace.selectedMarkerId = modelData.id; lane.workspace.markersVisible = true; lane.workspace.seekMarker(modelData.id); }
            }
        }
    }
}
