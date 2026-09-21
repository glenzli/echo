//! Navigation and precision editing for user-authored composition annotations.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: panel
    required property var workspace
    readonly property var selected: workspace.markers.find(item => item.id === workspace.selectedMarkerId) || null
    color: Theme.panel
    border.color: Theme.border

    function timeText(value: real): string {
        return workspace.formatTime(value) + "." + String(Math.round(value % 1000)).padStart(3, "0");
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 10
        Text { text: qsTr("Markers & ranges"); color: Theme.textPrimary; font.pixelSize: Theme.fontSection; font.weight: Font.DemiBold }
        RowLayout {
            Layout.fillWidth: true
            EchoButton {
                objectName: "addPointMarker"
                Layout.fillWidth: true
                text: qsTr("At playhead")
                ghost: true
                enabled: panel.workspace.markers.length < 256
                onClicked: panel.workspace.addMarker(false)
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Add marker (M)")
            }
            EchoButton {
                objectName: "addRangeMarker"
                Layout.fillWidth: true
                text: qsTr("From selection")
                ghost: true
                enabled: panel.workspace.markers.length < 256 && panel.workspace.selectionBounds !== null
                onClicked: panel.workspace.addMarker(true)
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Name selection range (Shift+M)")
            }
        }
        Text {
            visible: panel.workspace.markers.length === 0
            Layout.fillWidth: true
            text: qsTr("Mark moments to revisit, or name a range for repeated listening.")
            color: Theme.textMuted
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }
        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 80
            clip: true
            spacing: 3
            model: panel.workspace.markers
            ScrollBar.vertical: ScrollBar {}
            delegate: ItemDelegate {
                required property var modelData
                width: list.width
                height: 54
                objectName: "marker-" + modelData.id
                highlighted: panel.workspace.selectedMarkerId === modelData.id
                onClicked: {
                    panel.workspace.selectedMarkerId = modelData.id;
                    panel.workspace.seekMarker(modelData.id);
                }
                background: Rectangle { radius: 5; color: parent.highlighted ? Theme.accentSurface : parent.hovered ? Theme.controlQuiet : "transparent" }
                contentItem: Column {
                    spacing: 4
                    Text { width: parent.width; text: modelData.name; color: Theme.textPrimary; font.pixelSize: Theme.fontSection; elide: Text.ElideRight }
                    Text {
                        width: parent.width
                        text: modelData.startMillis > panel.workspace.durationMillis || (modelData.endMillis || 0) > panel.workspace.durationMillis
                              ? qsTr("Outside current audio")
                              : panel.timeText(modelData.startMillis) + (modelData.endMillis !== undefined ? " — " + panel.timeText(modelData.endMillis) : "")
                        color: Theme.textMuted; font.pixelSize: Theme.fontMeta; elide: Text.ElideRight
                    }
                }
                Accessible.name: modelData.name
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            visible: panel.selected !== null
            spacing: 8
            EchoTextField {
                objectName: "markerName"
                Layout.fillWidth: true
                text: panel.selected ? panel.selected.name : ""
                maximumLength: 120
                Accessible.name: qsTr("Marker name")
                onEditingFinished: {
                    if (!panel.selected) return;
                    if (text.trim().length) panel.workspace.patchMarker(panel.selected.id, "name", text.trim());
                    else text = panel.selected.name;
                }
            }
            RowLayout {
                Layout.fillWidth: true
                Text { Layout.fillWidth: true; text: qsTr("Start"); color: Theme.textSecondary; font.pixelSize: Theme.fontSection }
                EchoTimeSpinBox {
                    objectName: "markerStart"
                    Layout.preferredWidth: 128
                    from: 0
                    to: panel.selected && panel.selected.endMillis !== undefined ? panel.selected.endMillis - 1 : 14400000
                    value: panel.selected ? panel.selected.startMillis : 0
                    Accessible.name: qsTr("Marker start (seconds)")
                    onValueModified: if (panel.selected) panel.workspace.patchMarker(panel.selected.id, "startMillis", value)
                }
            }
            RowLayout {
                Layout.fillWidth: true
                visible: panel.selected !== null && panel.selected.endMillis !== undefined
                Text { Layout.fillWidth: true; text: qsTr("End"); color: Theme.textSecondary; font.pixelSize: Theme.fontSection }
                EchoTimeSpinBox {
                    objectName: "markerEnd"
                    Layout.preferredWidth: 128
                    from: panel.selected ? panel.selected.startMillis + 1 : 1
                    to: 14400000
                    value: panel.selected ? panel.selected.endMillis || from : from
                    Accessible.name: qsTr("Marker end (seconds)")
                    onValueModified: if (panel.selected) panel.workspace.patchMarker(panel.selected.id, "endMillis", value)
                }
            }
            RowLayout {
                Layout.fillWidth: true
                EchoButton {
                    objectName: "previewMarkerRange"
                    Layout.fillWidth: true
                    text: qsTr("Preview range")
                    enabled: panel.selected !== null && panel.selected.endMillis > panel.selected.startMillis && panel.selected.endMillis <= panel.workspace.durationMillis
                    onClicked: panel.workspace.previewMarker(panel.selected.id)
                }
                EchoButton {
                    objectName: "deleteMarker"
                    text: qsTr("Delete")
                    ghost: true
                    onClicked: if (panel.selected) panel.workspace.deleteMarker(panel.selected.id)
                }
            }
        }
        Text {
            Layout.fillWidth: true
            text: qsTr("Markers stay at their timeline positions when clips move. Alt+← / → jumps between markers.")
            color: Theme.textMuted
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }
    }
}
