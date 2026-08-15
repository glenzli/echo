//! Fixed-latency source-buffer beat repeat and reverse controls.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel
    required property var draft
    readonly property var family: draft.creativeVfxFamily("beatRepeat")
    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("beatRepeat");
        next[name] = value;
        draft.setCreativeVfxFamily("beatRepeat", next);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        RowLayout {
            Layout.fillWidth: true; Layout.preferredHeight: Theme.editorPanelHeaderHeight
            Layout.leftMargin: 12; Layout.rightMargin: 8; spacing: 7
            EchoIcon { source: "qrc:/EchoDesktop/icons/sparkles.svg"; size: 15; color: Theme.textSecondary }
            Text { text: qsTr("Beat Repeat"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Text { text: qsTr("Repeat or reverse captured source slices"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoSwitch { objectName: "beatRepeatVfxEnabled"; checked: Boolean(panel.family.enabled); accessibleName: qsTr("Beat Repeat"); onToggled: panel.draft.setCreativeVfxFamilyEnabled("beatRepeat", checked) }
        }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }
        GridLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.margins: 14
            columns: 2; columnSpacing: 24; rowSpacing: 12
            EchoParameterSlider { objectName: "beatRepeatVfxMix"; Layout.preferredWidth: 300; label: qsTr("Mix"); from: 0; to: 100; stepSize: 1; value: Number(panel.family.mixPercent || 0); valueText: Math.round(value) + "%"; onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture() }
            EchoParameterSlider { objectName: "beatRepeatVfxSlice"; Layout.preferredWidth: 300; label: qsTr("Slice"); from: 30; to: 500; stepSize: 5; value: Number(panel.family.sliceMillis || 30); valueText: Math.round(value) + " ms"; onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("sliceMillis", Math.round(value)); onGestureFinished: panel.draft.endGesture() }
            EchoParameterSlider { objectName: "beatRepeatVfxRepeats"; Layout.preferredWidth: 300; label: qsTr("Repeats"); from: 1; to: 4; stepSize: 1; value: Number(panel.family.repeatCount || 1); valueText: Math.round(value); onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("repeatCount", Math.round(value)); onGestureFinished: panel.draft.endGesture() }
            EchoSwitch { objectName: "beatRepeatVfxReverse"; Layout.alignment: Qt.AlignVCenter; checked: Boolean(panel.family.reverse); accessibleName: qsTr("Reverse slice"); onToggled: panel.updateValue("reverse", checked) }
        }
    }
}
