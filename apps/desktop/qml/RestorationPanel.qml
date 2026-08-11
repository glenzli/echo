//! Real-time controls for Echo's CPU restoration chain.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel
    required property var draft

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function toggle(enabled: bool, callback: var): void {
        panel.draft.beginGesture();
        callback(!enabled);
        panel.draft.endGesture();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            Layout.leftMargin: 12
            Layout.rightMargin: 8
            spacing: 7

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/high-pass.svg"
                size: 15
                color: Theme.textSecondary
            }
            Text {
                text: qsTr("Restoration")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Item {
                Layout.fillWidth: true
            }
            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset restoration")
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetRestoration()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 10
            Layout.bottomMargin: 10
            spacing: Theme.editorPanelGap

            DePlosivePanel {
                draft: panel.draft
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            ColumnLayout {
                Layout.preferredWidth: 230
                Layout.minimumWidth: 175
                Layout.maximumWidth: 300
                Layout.fillHeight: true
                spacing: 1

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: qsTr("Adaptive noise reduction")
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    EchoSwitch {
                        checked: panel.draft.noiseReductionEnabled
                        accessibleName: qsTr("Adaptive noise reduction")
                        onToggled: panel.toggle(panel.draft.noiseReductionEnabled, enabled => panel.draft.noiseReductionEnabled = enabled)
                    }
                }

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Reduction")
                    from: 0
                    to: 2400
                    stepSize: 50
                    value: panel.draft.noiseReductionCentibels
                    valueText: (value / 100).toFixed(1) + " dB"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setRestorationParameter("noiseReduction", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Sensitivity")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: panel.draft.noiseReductionSensitivityPercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setRestorationParameter("noiseSensitivity", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Release")
                    from: 20
                    to: 1000
                    stepSize: 10
                    value: panel.draft.noiseReductionSmoothingMillis
                    valueText: Math.round(value) + " ms"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setRestorationParameter("noiseSmoothing", value)
                    onGestureFinished: panel.draft.endGesture()
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Smoothly lowers steady background noise between foreground sounds.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            ColumnLayout {
                Layout.preferredWidth: 230
                Layout.minimumWidth: 175
                Layout.maximumWidth: 300
                Layout.fillHeight: true
                spacing: 1

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: qsTr("De-esser")
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                    }
                    Item {
                        Layout.fillWidth: true
                    }
                    EchoSwitch {
                        checked: panel.draft.deEsserEnabled
                        accessibleName: qsTr("De-esser")
                        onToggled: panel.toggle(panel.draft.deEsserEnabled, enabled => panel.draft.deEsserEnabled = enabled)
                    }
                }

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Frequency")
                    from: 3000
                    to: 12000
                    stepSize: 100
                    value: panel.draft.deEsserFrequencyHertz
                    valueText: (value / 1000).toFixed(1) + " kHz"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setRestorationParameter("deEsserFrequency", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Threshold")
                    from: -6000
                    to: 0
                    stepSize: 50
                    value: panel.draft.deEsserThresholdCentibels
                    valueText: (value / 100).toFixed(1) + " dB"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setRestorationParameter("deEsserThreshold", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Limit")
                    from: 0
                    to: 1800
                    stepSize: 50
                    value: panel.draft.deEsserReductionCentibels
                    valueText: (value / 100).toFixed(1) + " dB"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setRestorationParameter("deEsserReduction", value)
                    onGestureFinished: panel.draft.endGesture()
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Softens harsh sibilance without turning down the whole voice.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }
            }

            Item {
                Layout.fillWidth: true
            }
        }
    }
}
