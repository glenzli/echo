//! Antialiased creative saturation. This panel edits only authored Drive
//! intent; processor latency and oversampling remain owned by the audio engine.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("drive")
    readonly property int character: Number(family.character || 0)

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("drive");
        next[name] = value;
        draft.setCreativeVfxFamily("drive", next);
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
                text: qsTr("Drive")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Text {
                text: qsTr("Antialiased saturation")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }
            Item { Layout.fillWidth: true }
            EchoSwitch {
                objectName: "driveVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Drive")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("drive", checked)
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
                objectName: "driveVfxCharacterSelector"
                Layout.preferredWidth: 480
                model: [qsTr("Soft clip"), qsTr("Overdrive"), qsTr("Fuzz")]
                currentIndex: panel.character
                onActivated: index => panel.updateValue("character", index)
            }

            GridLayout {
                Layout.preferredWidth: 650
                columns: 2
                columnSpacing: 24
                rowSpacing: 12

                EchoParameterSlider {
                    objectName: "driveVfxMix"
                    Layout.preferredWidth: 300
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
                    objectName: "driveVfxAmount"
                    Layout.preferredWidth: 300
                    label: qsTr("Drive")
                    from: 0
                    to: 3600
                    stepSize: 25
                    value: Number(panel.family.driveCentibels || 0)
                    valueText: (value / 100).toFixed(1) + " dB"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("driveCentibels", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }

                EchoParameterSlider {
                    objectName: "driveVfxTone"
                    Layout.preferredWidth: 300
                    label: qsTr("Tone")
                    from: 500
                    to: 16000
                    stepSize: 100
                    value: Number(panel.family.toneHertz || 500)
                    valueText: value >= 1000 ? (value / 1000).toFixed(1) + " kHz" : Math.round(value) + " Hz"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("toneHertz", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }

                EchoParameterSlider {
                    objectName: "driveVfxOutput"
                    Layout.preferredWidth: 300
                    label: qsTr("Output")
                    from: -2400
                    to: 600
                    stepSize: 25
                    value: Number(panel.family.outputGainCentibels || 0)
                    valueText: (value >= 0 ? "+" : "") + (value / 100).toFixed(1) + " dB"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("outputGainCentibels", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }
            }

            Text {
                Layout.preferredWidth: 650
                text: qsTr("Input-driven saturation with a compensated antialiasing path. Characters are listening styles, not models of named hardware.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            Item { Layout.fillHeight: true }
        }
    }
}
