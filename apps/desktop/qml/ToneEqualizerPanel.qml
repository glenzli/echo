//! Compact six-band parametric equalizer. The draft owns authored values;
//! ParametricEqGraph owns direct manipulation and response presentation.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    required property var responseProvider
    readonly property var selected: draft.equalizerBands[graph.selectedBand]

    implicitWidth: 320
    implicitHeight: 316
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    function formatGain(centibels: int) : string {
        const decibels = centibels / 100
        return (decibels >= 0 ? "+" : "") + decibels.toFixed(1) + " dB"
    }

    function formatFrequency(hertz: int) : string {
        return hertz >= 1000 ? (hertz / 1000).toFixed(hertz < 10000 ? 1 : 0)
            + " kHz" : hertz + " Hz"
    }

    function editSelected(enabled: bool, filterKind: int,
                          frequencyHertz: int, qHundredths: int,
                          gainCentibels: int) : void {
        draft.setEqualizerBand(graph.selectedBand, enabled, filterKind,
            frequencyHertz, qHundredths, gainCentibels)
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 31
            Layout.leftMargin: 10
            Layout.rightMargin: 7
            spacing: 6

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/equalizer.svg"
                size: 15
                color: Theme.textSecondary
            }
            Text {
                text: qsTr("Parametric EQ")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Item { Layout.fillWidth: true }
            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset equalizer")
                enabled: !panel.draft.equalizerIsFlat()
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetEqualizer()
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ParametricEqGraph {
            id: graph
            Layout.fillWidth: true
            Layout.preferredHeight: 118
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 7
            Layout.bottomMargin: 5
            draft: panel.draft
            responseProvider: panel.responseProvider
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 29
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            spacing: 6

            CheckBox {
                checked: Boolean(panel.selected.enabled)
                text: qsTr("Band %1").arg(graph.selectedBand + 1)
                onToggled: panel.editSelected(checked,
                    Number(panel.selected.filterKind),
                    Number(panel.selected.frequencyHertz),
                    Number(panel.selected.qHundredths),
                    Number(panel.selected.gainCentibels))
            }
            Item { Layout.fillWidth: true }
            ComboBox {
                Layout.preferredWidth: 108
                model: [qsTr("Bell"), qsTr("Low shelf"),
                        qsTr("High shelf"), qsTr("Notch")]
                currentIndex: Number(panel.selected.filterKind)
                onActivated: index => panel.editSelected(
                    Boolean(panel.selected.enabled), index,
                    Number(panel.selected.frequencyHertz),
                    Number(panel.selected.qHundredths),
                    Number(panel.selected.gainCentibels))
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 5
            Layout.bottomMargin: 6
            spacing: 2

            EchoParameterSlider {
                Layout.fillWidth: true
                label: qsTr("Frequency")
                from: 0
                to: 1000
                stepSize: 1
                value: Math.log(Number(panel.selected.frequencyHertz) / 20)
                    / Math.log(1000) * 1000
                valueWidth: 64
                valueText: panel.formatFrequency(Math.round(
                    20 * Math.pow(1000, value / 1000)))
                onGestureStarted: panel.draft.beginGesture()
                onGestureFinished: panel.draft.endGesture()
                onEdited: sliderValue => panel.editSelected(
                    Boolean(panel.selected.enabled), Number(panel.selected.filterKind),
                    Math.round(20 * Math.pow(1000, sliderValue / 1000)),
                    Number(panel.selected.qHundredths),
                    Number(panel.selected.gainCentibels))
            }
            EchoParameterSlider {
                Layout.fillWidth: true
                label: qsTr("Q")
                from: 10
                to: 2000
                stepSize: 5
                value: Number(panel.selected.qHundredths)
                valueWidth: 64
                valueText: (value / 100).toFixed(2)
                onGestureStarted: panel.draft.beginGesture()
                onGestureFinished: panel.draft.endGesture()
                onEdited: sliderValue => panel.editSelected(
                    Boolean(panel.selected.enabled), Number(panel.selected.filterKind),
                    Number(panel.selected.frequencyHertz), Math.round(sliderValue),
                    Number(panel.selected.gainCentibels))
            }
            EchoParameterSlider {
                Layout.fillWidth: true
                label: qsTr("Gain")
                enabled: Number(panel.selected.filterKind) !== 3
                from: -1200
                to: 1200
                stepSize: 10
                value: Number(panel.selected.gainCentibels)
                valueWidth: 64
                neutralValue: 0
                showNeutralMarker: true
                valueText: panel.formatGain(Math.round(value))
                onGestureStarted: panel.draft.beginGesture()
                onGestureFinished: panel.draft.endGesture()
                onEdited: sliderValue => panel.editSelected(
                    Boolean(panel.selected.enabled), Number(panel.selected.filterKind),
                    Number(panel.selected.frequencyHertz), Number(panel.selected.qHundredths),
                    Math.round(sliderValue))
            }
        }
    }
}
