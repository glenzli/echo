//! Authored short-impulse repair controls. The draft owns validation,
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
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    function toggleEnabled(): void {
        draft.beginGesture()
        draft.setDeClickEnabled(!draft.deClickEnabled)
        draft.endGesture()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 30
            Layout.leftMargin: 10
            Layout.rightMargin: 7
            spacing: 7

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("De-click")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Item { Layout.fillWidth: true }

            Text {
                text: qsTr("Enabled")
                color: panel.draft.deClickEnabled
                    ? Theme.textSecondary : Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }

            Rectangle {
                implicitWidth: 30
                implicitHeight: 17
                radius: height / 2
                color: panel.draft.deClickEnabled ? Theme.accent : Theme.track

                Rectangle {
                    width: 13
                    height: 13
                    radius: 7
                    y: 2
                    x: panel.draft.deClickEnabled ? parent.width - width - 2 : 2
                    color: panel.draft.deClickEnabled
                        ? Theme.accentText : Theme.panelRaised

                    Behavior on x { NumberAnimation { duration: 90 } }
                }

                TapHandler { onTapped: panel.toggleEnabled() }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset de-click")
                enabled: panel.draft.deClickEnabled
                    || panel.draft.deClickSensitivityPercent !== 50
                    || panel.draft.deClickMaximumClickMicroseconds !== 1000
                    || panel.draft.deClickRepairPercent !== 100
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetDeClick()
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
            Layout.topMargin: 12
            Layout.bottomMargin: 8
            spacing: 5

            EchoParameterSlider {
                Layout.fillWidth: true
                labelWidth: 98
                label: qsTr("Sensitivity")
                from: 0
                to: 100
                stepSize: 1
                value: panel.draft.deClickSensitivityPercent
                valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setDeClickParameter("sensitivity", value)
                onGestureFinished: panel.draft.endGesture()
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                labelWidth: 98
                valueWidth: 66
                label: qsTr("Maximum impulse")
                from: 50
                to: 2000
                stepSize: 10
                value: panel.draft.deClickMaximumClickMicroseconds
                valueText: Math.round(value) + " µs"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setDeClickParameter("maximumClick", value)
                onGestureFinished: panel.draft.endGesture()
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                labelWidth: 98
                label: qsTr("Repair")
                from: 0
                to: 100
                stepSize: 1
                value: panel.draft.deClickRepairPercent
                valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setDeClickParameter("repair", value)
                onGestureFinished: panel.draft.endGesture()
            }

            Text {
                Layout.fillWidth: true
                Layout.topMargin: 5
                text: qsTr("Repairs isolated clicks and short impulses without smoothing sustained transients.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            Item { Layout.fillHeight: true }
        }
    }
}
