//! Generic rotary-speaker-inspired motion. This panel owns authored controls
//! only and makes no cabinet, microphone, or branded hardware claim.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("rotary")
    readonly property int speed: Number(family.speed || 0)

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("rotary");
        next[name] = value;
        draft.setCreativeVfxFamily("rotary", next);
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
                text: qsTr("Rotary")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Text {
                text: qsTr("Dual-rotor motion")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }
            Item { Layout.fillWidth: true }
            EchoSwitch {
                objectName: "rotaryVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Rotary")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("rotary", checked)
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
                objectName: "rotaryVfxSpeedSelector"
                Layout.preferredWidth: 420
                model: [qsTr("Slow"), qsTr("Fast"), qsTr("Brake")]
                currentIndex: panel.speed
                onActivated: index => panel.updateValue("speed", index)
            }

            GridLayout {
                Layout.preferredWidth: 650
                columns: 2
                columnSpacing: 24
                rowSpacing: 12

                EchoParameterSlider {
                    objectName: "rotaryVfxMix"
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
                    objectName: "rotaryVfxMotion"
                    Layout.preferredWidth: 300
                    label: qsTr("Motion")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: Number(panel.family.motionPercent || 0)
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("motionPercent", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }

                EchoParameterSlider {
                    objectName: "rotaryVfxWidth"
                    Layout.preferredWidth: 300
                    label: qsTr("Stereo width")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: Number(panel.family.stereoWidthPercent || 0)
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("stereoWidthPercent", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }
            }

            Text {
                Layout.preferredWidth: 650
                text: qsTr("Slow and Fast follow separate rotor inertia. Brake decelerates the motion instead of stopping it abruptly.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            Item { Layout.fillHeight: true }
        }
    }
}
