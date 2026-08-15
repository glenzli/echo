//! Zero-latency width and pan controls for existing stereo content.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel
    required property var draft
    readonly property var family: draft.creativeVfxFamily("stereo")
    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("stereo");
        next[name] = value;
        draft.setCreativeVfxFamily("stereo", next);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        RowLayout {
            Layout.fillWidth: true; Layout.preferredHeight: Theme.editorPanelHeaderHeight
            Layout.leftMargin: 12; Layout.rightMargin: 8; spacing: 7
            EchoIcon { source: "qrc:/EchoDesktop/icons/sparkles.svg"; size: 15; color: Theme.textSecondary }
            Text { text: qsTr("Stereo"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Text { text: qsTr("Width and pan"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoSwitch { objectName: "stereoVfxEnabled"; checked: Boolean(panel.family.enabled); accessibleName: qsTr("Stereo"); onToggled: panel.draft.setCreativeVfxFamilyEnabled("stereo", checked) }
        }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }
        GridLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.margins: 14
            columns: 2; columnSpacing: 24; rowSpacing: 12
            EchoParameterSlider { objectName: "stereoVfxMix"; Layout.preferredWidth: 300; label: qsTr("Mix"); from: 0; to: 100; stepSize: 1; value: Number(panel.family.mixPercent || 0); valueText: Math.round(value) + "%"; onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture() }
            EchoParameterSlider { objectName: "stereoVfxWidth"; Layout.preferredWidth: 300; label: qsTr("Width"); from: 0; to: 200; stepSize: 1; value: Number(panel.family.widthPercent || 0); valueText: Math.round(value) + "%"; onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("widthPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture() }
            EchoParameterSlider { objectName: "stereoVfxPan"; Layout.preferredWidth: 300; label: qsTr("Pan"); from: -100; to: 100; stepSize: 1; value: Number(panel.family.panPercent || 0); valueText: Math.round(value) === 0 ? qsTr("Center") : (value < 0 ? "L" : "R") + Math.abs(Math.round(value)); onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("panPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture() }
        }
    }
}
