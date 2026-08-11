//! Compact authored controls for Echo's real algorithmic room processor.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft

    implicitWidth: 560
    implicitHeight: 286
    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function frequency(hertz: int): string {
        return hertz >= 1000 ? (hertz / 1000).toFixed(hertz % 1000 === 0 ? 0 : 1) + " kHz" : hertz + " Hz";
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
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Algorithmic room")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Rectangle {
                implicitWidth: statusText.implicitWidth + 12
                implicitHeight: 18
                radius: 9
                color: panel.draft.reverbEnabled ? Theme.surfaceSelected : Theme.surfaceSubtle

                Text {
                    id: statusText
                    anchors.centerIn: parent
                    text: panel.draft.reverbEnabled ? qsTr("LIVE") : qsTr("BYPASS")
                    color: panel.draft.reverbEnabled ? Theme.accent : Theme.textDisabled
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.5
                }
            }

            Item {
                Layout.fillWidth: true
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset room")
                enabled: panel.draft.reverbEnabled || panel.draft.reverbMixPercent !== 18 || panel.draft.reverbPreDelayMillis !== 20 || panel.draft.reverbDecayMillis !== 1800 || panel.draft.reverbSizePercent !== 55 || panel.draft.reverbDampingPercent !== 45 || panel.draft.reverbLowCutHertz !== 120 || panel.draft.reverbHighCutHertz !== 10000
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetReverb()
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
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 7
            Layout.bottomMargin: 7
            spacing: 18

            ColumnLayout {
                Layout.preferredWidth: Math.min(340, (panel.width - 58) / 2)
                Layout.minimumWidth: 236
                Layout.maximumWidth: 360
                Layout.fillHeight: true
                spacing: 1

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Mix")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: panel.draft.reverbMixPercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("mix", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Pre-delay")
                    from: 0
                    to: 200
                    stepSize: 1
                    value: panel.draft.reverbPreDelayMillis
                    valueText: Math.round(value) + " ms"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("preDelay", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Decay")
                    from: 100
                    to: 12000
                    stepSize: 50
                    value: panel.draft.reverbDecayMillis
                    valueText: (value / 1000).toFixed(2) + " s"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("decay", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Size")
                    from: 10
                    to: 100
                    stepSize: 1
                    value: panel.draft.reverbSizePercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("size", value)
                    onGestureFinished: panel.draft.endGesture()
                }
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            ColumnLayout {
                Layout.preferredWidth: Math.min(340, (panel.width - 58) / 2)
                Layout.minimumWidth: 236
                Layout.maximumWidth: 360
                Layout.fillHeight: true
                spacing: 1

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Damping")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: panel.draft.reverbDampingPercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("damping", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Low cut")
                    from: 20
                    to: 1000
                    stepSize: 10
                    value: panel.draft.reverbLowCutHertz
                    valueText: panel.frequency(value)
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("lowCut", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("High cut")
                    from: 1000
                    to: 20000
                    stepSize: 100
                    value: panel.draft.reverbHighCutHertz
                    valueText: panel.frequency(value)
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("highCut", value)
                    onGestureFinished: panel.draft.endGesture()
                }

                Item {
                    Layout.fillHeight: true
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Insert effect · before fade and master")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    horizontalAlignment: Text.AlignRight
                }
            }

            Item {
                Layout.fillWidth: true
            }
        }
    }
}
