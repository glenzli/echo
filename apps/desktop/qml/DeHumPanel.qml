//! Authored power-line hum removal controls. The draft owns validation,
//! gesture history, and persistence; this component owns presentation only.

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

    function toggleEnabled(): void {
        draft.beginGesture();
        draft.setDeHumEnabled(!draft.deHumEnabled);
        draft.endGesture();
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
                text: qsTr("De-hum")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Item {
                Layout.fillWidth: true
            }

            EchoSwitch {
                checked: panel.draft.deHumEnabled
                accessibleName: qsTr("De-hum")
                onToggled: panel.toggleEnabled()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset de-hum")
                enabled: panel.draft.deHumEnabled || panel.draft.deHumFundamentalHertz !== 50 || panel.draft.deHumHarmonicCount !== 4 || panel.draft.deHumQualityTenths !== 300 || panel.draft.deHumDepthCentibels !== 2400
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetDeHum()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.maximumWidth: 620
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 9
            Layout.bottomMargin: 8
            spacing: 4

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 30
                spacing: 8

                Text {
                    Layout.preferredWidth: 70
                    text: qsTr("Mains")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSection
                    horizontalAlignment: Text.AlignRight
                }

                EchoButton {
                    implicitWidth: 62
                    implicitHeight: 26
                    text: "50 Hz"
                    ghost: true
                    selected: panel.draft.deHumFundamentalHertz === 50
                    onClicked: panel.draft.setDeHumParameter("fundamental", 50)
                }

                EchoButton {
                    implicitWidth: 62
                    implicitHeight: 26
                    text: "60 Hz"
                    ghost: true
                    selected: panel.draft.deHumFundamentalHertz === 60
                    onClicked: panel.draft.setDeHumParameter("fundamental", 60)
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                labelWidth: 70
                label: qsTr("Harmonics")
                from: 1
                to: 8
                stepSize: 1
                value: panel.draft.deHumHarmonicCount
                valueText: Math.round(value).toString()
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setDeHumParameter("harmonics", value)
                onGestureFinished: panel.draft.endGesture()
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                labelWidth: 70
                label: "Q"
                from: 50
                to: 1000
                stepSize: 10
                value: panel.draft.deHumQualityTenths
                valueText: (value / 10).toFixed(1)
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setDeHumParameter("quality", value)
                onGestureFinished: panel.draft.endGesture()
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                labelWidth: 70
                label: qsTr("Depth")
                from: 0
                to: 4800
                stepSize: 50
                value: panel.draft.deHumDepthCentibels
                valueText: (value / 100).toFixed(1) + " dB"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setDeHumParameter("depth", value)
                onGestureFinished: panel.draft.endGesture()
            }

            Text {
                Layout.fillWidth: true
                Layout.topMargin: 3
                text: qsTr("Notches the selected mains frequency and its harmonics while preserving nearby sound.")
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
