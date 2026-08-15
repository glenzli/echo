//! Fixed-latency pitch, harmony, and formant-colour controls. Formant colour
//! is an intentional spectral colour control, not a voice-identity claim.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("pitch")

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("pitch");
        next[name] = value;
        draft.setCreativeVfxFamily("pitch", next);
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
            Text { text: qsTr("Pitch"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Text { text: qsTr("Harmony and formant colour"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoSwitch {
                objectName: "pitchVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Pitch")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("pitch", checked)
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            spacing: 12

            GridLayout {
                Layout.preferredWidth: 650
                columns: 2
                columnSpacing: 24
                rowSpacing: 12

                EchoParameterSlider {
                    objectName: "pitchVfxMix"; Layout.preferredWidth: 300; label: qsTr("Mix"); from: 0; to: 100; stepSize: 1
                    value: Number(panel.family.mixPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    objectName: "pitchVfxSemitones"; Layout.preferredWidth: 300; label: qsTr("Pitch shift"); from: -12; to: 12; stepSize: 1
                    value: Number(panel.family.pitchSemitones || 0); valueText: (value > 0 ? "+" : "") + Math.round(value) + " st"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("pitchSemitones", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    objectName: "pitchVfxHarmonySemitones"; Layout.preferredWidth: 300; label: qsTr("Harmony interval"); from: -12; to: 12; stepSize: 1
                    value: Number(panel.family.harmonySemitones || 0); valueText: (value > 0 ? "+" : "") + Math.round(value) + " st"
                    enabled: Boolean(panel.family.harmonyEnabled)
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("harmonySemitones", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    objectName: "pitchVfxHarmonyMix"; Layout.preferredWidth: 300; label: qsTr("Harmony mix"); from: 0; to: 100; stepSize: 1
                    value: Number(panel.family.harmonyMixPercent || 0); valueText: Math.round(value) + "%"
                    enabled: Boolean(panel.family.harmonyEnabled)
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("harmonyMixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    objectName: "pitchVfxFormantColour"; Layout.preferredWidth: 300; label: qsTr("Formant colour"); from: -12; to: 12; stepSize: 1
                    value: Number(panel.family.formantColourSemitones || 0); valueText: (value > 0 ? "+" : "") + Math.round(value) + " st"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateValue("formantColourSemitones", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoSwitch {
                    objectName: "pitchVfxHarmonyEnabled"; Layout.alignment: Qt.AlignVCenter
                    checked: Boolean(panel.family.harmonyEnabled); accessibleName: qsTr("Harmony")
                    onToggled: panel.updateValue("harmonyEnabled", checked)
                }
            }

            Text { Layout.preferredWidth: 650; text: qsTr("Pitch uses a fixed causal delay. Formant colour changes spectral character; it does not preserve or imitate a specific voice."); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
            Item { Layout.fillHeight: true }
        }
    }
}
