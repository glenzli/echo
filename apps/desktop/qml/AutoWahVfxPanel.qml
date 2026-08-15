//! Envelope-following resonant filter. The DSP follows the input; no model or
//! generated source is involved.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("autoWah")

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("autoWah");
        next[name] = value;
        draft.setCreativeVfxFamily("autoWah", next);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.editorPanelHeaderHeight
            Layout.leftMargin: 12
            Layout.rightMargin: 8
            spacing: 7

            EchoIcon { source: "qrc:/EchoDesktop/icons/sparkles.svg"; size: 15; color: Theme.textSecondary }
            Text { text: qsTr("Auto-Wah"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Text { text: qsTr("Envelope-following filter"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoSwitch {
                objectName: "autoWahVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Auto-Wah")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("autoWah", checked)
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            columns: 2
            columnSpacing: 24
            rowSpacing: 12

            EchoParameterSlider {
                objectName: "autoWahVfxMix"; Layout.preferredWidth: 300; label: qsTr("Mix"); from: 0; to: 100; stepSize: 1
                value: Number(panel.family.mixPercent || 0); valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "autoWahVfxSensitivity"; Layout.preferredWidth: 300; label: qsTr("Sensitivity"); from: 0; to: 100; stepSize: 1
                value: Number(panel.family.sensitivityPercent || 0); valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("sensitivityPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "autoWahVfxMinimumFrequency"; Layout.preferredWidth: 300; label: qsTr("Low frequency"); from: 80; to: 8000; stepSize: 10
                value: Number(panel.family.minimumFrequencyHertz || 80); valueText: Math.round(value) + " Hz"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("minimumFrequencyHertz", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "autoWahVfxMaximumFrequency"; Layout.preferredWidth: 300; label: qsTr("High frequency"); from: Math.max(81, Number(panel.family.minimumFrequencyHertz || 80) + 1); to: 16000; stepSize: 10
                value: Math.max(from, Number(panel.family.maximumFrequencyHertz || from)); valueText: Math.round(value) + " Hz"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("maximumFrequencyHertz", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "autoWahVfxResonance"; Layout.preferredWidth: 300; label: qsTr("Resonance"); from: 5; to: 50; stepSize: 1
                value: Number(panel.family.resonanceTenths || 5); valueText: (value / 10).toFixed(1)
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("resonanceTenths", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
        }
    }
}
