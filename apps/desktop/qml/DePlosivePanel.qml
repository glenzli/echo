//! Focused controls for low-frequency speech-plosive suppression.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

ColumnLayout {
    id: panel
    required property var draft

    Layout.preferredWidth: 230
    Layout.minimumWidth: 175
    Layout.maximumWidth: 300
    Layout.fillHeight: true
    spacing: 1

    RowLayout {
        Layout.fillWidth: true
        Text {
            text: qsTr("De-plosive")
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
            font.weight: Font.DemiBold
        }
        Item { Layout.fillWidth: true }
        EchoIconButton {
            source: "qrc:/EchoDesktop/icons/reset-all.svg"
            toolTipText: qsTr("Reset de-plosive")
            buttonSize: 23
            iconSize: 13
            onClicked: panel.draft.resetDePlosive()
        }
        Rectangle {
            implicitWidth: 30
            implicitHeight: 17
            radius: height / 2
            color: panel.draft.dePlosiveEnabled ? Theme.accent : Theme.track
            Rectangle {
                width: 13
                height: 13
                radius: 7
                y: 2
                x: panel.draft.dePlosiveEnabled ? parent.width - width - 2 : 2
                color: panel.draft.dePlosiveEnabled ? Theme.accentText : Theme.panelRaised
                Behavior on x { NumberAnimation { duration: 90 } }
            }
            TapHandler {
                onTapped: {
                    panel.draft.beginGesture();
                    panel.draft.setDePlosiveEnabled(!panel.draft.dePlosiveEnabled);
                    panel.draft.endGesture();
                }
            }
        }
    }

    EchoParameterSlider {
        Layout.fillWidth: true
        label: qsTr("Band edge")
        from: 80
        to: 240
        stepSize: 5
        value: panel.draft.dePlosiveFrequencyHertz
        valueText: Math.round(value) + " Hz"
        onGestureStarted: panel.draft.beginGesture()
        onEdited: value => panel.draft.setDePlosiveParameter("frequency", value)
        onGestureFinished: panel.draft.endGesture()
    }
    EchoParameterSlider {
        Layout.fillWidth: true
        label: qsTr("Sensitivity")
        from: 0
        to: 100
        stepSize: 1
        value: panel.draft.dePlosiveSensitivityPercent
        valueText: Math.round(value) + "%"
        onGestureStarted: panel.draft.beginGesture()
        onEdited: value => panel.draft.setDePlosiveParameter("sensitivity", value)
        onGestureFinished: panel.draft.endGesture()
    }
    EchoParameterSlider {
        Layout.fillWidth: true
        label: qsTr("Reduction")
        from: 0
        to: 1800
        stepSize: 50
        value: panel.draft.dePlosiveReductionCentibels
        valueText: (value / 100).toFixed(1) + " dB"
        onGestureStarted: panel.draft.beginGesture()
        onEdited: value => panel.draft.setDePlosiveParameter("reduction", value)
        onGestureFinished: panel.draft.endGesture()
    }
    EchoParameterSlider {
        Layout.fillWidth: true
        label: qsTr("Release")
        from: 40
        to: 500
        stepSize: 10
        value: panel.draft.dePlosiveReleaseMillis
        valueText: Math.round(value) + " ms"
        onGestureStarted: panel.draft.beginGesture()
        onEdited: value => panel.draft.setDePlosiveParameter("release", value)
        onGestureFinished: panel.draft.endGesture()
    }

    Text {
        Layout.fillWidth: true
        text: qsTr("Reduces short microphone pops below the band edge while preserving the rest of the voice.")
        color: Theme.textDisabled
        font.pixelSize: Theme.fontMeta
        wrapMode: Text.WordWrap
    }
}
