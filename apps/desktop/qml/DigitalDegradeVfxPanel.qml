//! Input-driven digital resolution effects. This owner edits only the typed
//! Digital Degrade family; it does not generate noise or mutate the source.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("digitalDegrade")
    readonly property int character: Number(family.character || 0)
    readonly property var bitcrusher: family.bitcrusher || ({})
    readonly property var sampleRateReduction: family.sampleRateReduction || ({})

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("digitalDegrade");
        next[name] = value;
        draft.setCreativeVfxFamily("digitalDegrade", next);
    }

    function updateNested(group: string, name: string, value: int): void {
        const next = draft.creativeVfxFamily("digitalDegrade");
        const parameters = draft.copyCreativeVfx(next[group]);
        parameters[name] = value;
        next[group] = parameters;
        draft.setCreativeVfxFamily("digitalDegrade", next);
    }

    function rate(value: real): string {
        return value >= 1000 ? (value / 1000).toFixed(value % 1000 === 0 ? 0 : 1) + " kHz" : Math.round(value) + " Hz";
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

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/sparkles.svg"
                size: 15
                color: Theme.textSecondary
            }
            Text {
                text: qsTr("Digital Degrade")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Text {
                text: qsTr("Resolution and sample-rate character")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }
            Item {
                Layout.fillWidth: true
            }
            EchoSwitch {
                objectName: "digitalDegradeEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Digital Degrade")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("digitalDegrade", checked)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            spacing: 14

            EchoSegmentedControl {
                objectName: "digitalDegradeCharacterSelector"
                Layout.fillWidth: true
                Layout.maximumWidth: 520
                model: [qsTr("Bitcrusher"), qsTr("Sample-rate reduction"), qsTr("Lo-Fi")]
                currentIndex: panel.character
                onActivated: index => panel.updateValue("character", index)
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.maximumWidth: 680
                spacing: 22

                EchoParameterSlider {
                    objectName: "digitalDegradeMix"
                    Layout.fillWidth: true
                    label: qsTr("Mix")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: Number(panel.family.mixPercent || 0)
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("mixPercent", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }

                EchoParameterSlider {
                    objectName: "digitalDegradeBitDepth"
                    visible: panel.character === 0 || panel.character === 2
                    enabled: visible
                    Layout.fillWidth: true
                    label: qsTr("Bit depth")
                    from: 2
                    to: 16
                    stepSize: 1
                    value: Number(panel.bitcrusher.bitDepth || 0)
                    valueText: Math.round(value) + " bit"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateNested("bitcrusher", "bitDepth", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }

                EchoParameterSlider {
                    objectName: "digitalDegradeTargetRate"
                    visible: panel.character === 1 || panel.character === 2
                    enabled: visible
                    Layout.fillWidth: true
                    label: qsTr("Target rate")
                    from: 1000
                    to: 24000
                    stepSize: 100
                    value: Number(panel.sampleRateReduction.targetRateHertz || 0)
                    valueText: panel.rate(value)
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateNested("sampleRateReduction", "targetRateHertz", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }
            }

            Text {
                Layout.fillWidth: true
                Layout.maximumWidth: 680
                text: qsTr("Reduces digital resolution using the source signal only. It does not add hiss or generated ambience.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }
}
