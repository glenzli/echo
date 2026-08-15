//! Input-driven tape colour. The engine owns the modulation and dropout state;
//! this panel persists only the intentional style controls.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("tape")

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("tape");
        next[name] = value;
        draft.setCreativeVfxFamily("tape", next);
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
            Text { text: qsTr("Tape"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Text { text: qsTr("Input-driven colour"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoSwitch {
                objectName: "tapeVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Tape")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("tape", checked)
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
                objectName: "tapeVfxMix"; Layout.preferredWidth: 300; label: qsTr("Mix"); from: 0; to: 100; stepSize: 1
                value: Number(panel.family.mixPercent || 0); valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "tapeVfxSaturation"; Layout.preferredWidth: 300; label: qsTr("Saturation"); from: 0; to: 100; stepSize: 1
                value: Number(panel.family.saturationPercent || 0); valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("saturationPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "tapeVfxWowFlutter"; Layout.preferredWidth: 300; label: qsTr("Wow & flutter"); from: 0; to: 100; stepSize: 1
                value: Number(panel.family.wowFlutterPercent || 0); valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("wowFlutterPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                objectName: "tapeVfxDropout"; Layout.preferredWidth: 300; label: qsTr("Dropout"); from: 0; to: 100; stepSize: 1
                value: Number(panel.family.dropoutPercent || 0); valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("dropoutPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
            }
        }
    }
}
