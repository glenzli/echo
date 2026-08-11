//! Focused six-band parametric equalizer. The response stays the primary
//! canvas while the selected band receives a compact, bounded inspector.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    required property var responseProvider
    readonly property var selected: draft.equalizerBands[graph.selectedBand]

    implicitWidth: 780
    implicitHeight: 320
    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function formatGain(centibels: int): string {
        const decibels = centibels / 100;
        return (decibels >= 0 ? "+" : "") + decibels.toFixed(1) + " dB";
    }

    function formatFrequency(hertz: int): string {
        return hertz >= 1000 ? (hertz / 1000).toFixed(hertz < 10000 ? 1 : 0) + " kHz" : hertz + " Hz";
    }

    function editSelected(enabled: bool, filterKind: int, frequencyHertz: int, qHundredths: int, gainCentibels: int): void {
        draft.setEqualizerBand(graph.selectedBand, enabled, filterKind, frequencyHertz, qHundredths, gainCentibels);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            Layout.leftMargin: 14
            Layout.rightMargin: 9
            spacing: 8

            Rectangle {
                Layout.preferredWidth: 26
                Layout.preferredHeight: 26
                radius: 8
                color: Theme.accentSurfaceQuiet

                EchoIcon {
                    anchors.centerIn: parent
                    source: "qrc:/EchoDesktop/icons/equalizer.svg"
                    size: 15
                    color: Theme.accentSelectionText
                }
            }

            ColumnLayout {
                spacing: -1
                Text {
                    text: qsTr("Parametric EQ")
                    color: Theme.textPrimary
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                }
                Text {
                    text: qsTr("6-band parametric")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontMeta
                }
            }

            Item {
                Layout.fillWidth: true
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset equalizer")
                enabled: !panel.draft.equalizerIsFlat()
                buttonSize: 28
                iconSize: 14
                onClicked: panel.draft.resetEqualizer()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.bottomMargin: 10
            radius: Theme.controlRadius
            color: Theme.parameterGraph
            border.width: 1
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 12

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.minimumWidth: 420

                    ParametricEqGraph {
                        id: graph
                        anchors.fill: parent
                        draft: panel.draft
                        responseProvider: panel.responseProvider
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 242
                    Layout.fillHeight: true
                    radius: Theme.controlRadius
                    color: Theme.parameterSection
                    border.width: 1
                    border.color: Theme.border

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 6

                        RowLayout {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 26
                            spacing: 7

                            Rectangle {
                                Layout.preferredWidth: 22
                                Layout.preferredHeight: 22
                                radius: 11
                                color: panel.selected.enabled ? Theme.accent : Theme.surfaceSubtle
                                border.width: panel.selected.enabled ? 0 : 1
                                border.color: Theme.borderStrong

                                Text {
                                    anchors.centerIn: parent
                                    text: graph.selectedBand + 1
                                    color: panel.selected.enabled ? Theme.accentText : Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    font.weight: Font.DemiBold
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: qsTr("Band %1").arg(graph.selectedBand + 1)
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.weight: Font.DemiBold
                            }

                            EchoSwitch {
                                checked: Boolean(panel.selected.enabled)
                                accessibleName: qsTr("Band %1").arg(graph.selectedBand + 1)
                                onToggled: checked => panel.editSelected(checked, Number(panel.selected.filterKind), Number(panel.selected.frequencyHertz), Number(panel.selected.qHundredths), Number(panel.selected.gainCentibels))
                            }
                        }

                        EchoSegmentedControl {
                            Layout.fillWidth: true
                            model: [qsTr("Bell"), qsTr("Low shelf"), qsTr("High shelf"), qsTr("Notch")]
                            currentIndex: Number(panel.selected.filterKind)
                            onActivated: index => panel.editSelected(Boolean(panel.selected.enabled), index, Number(panel.selected.frequencyHertz), Number(panel.selected.qHundredths), Number(panel.selected.gainCentibels))
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 1
                            Layout.topMargin: 2
                            Layout.bottomMargin: 2
                            color: Theme.border
                        }

                        EchoParameterSlider {
                            Layout.fillWidth: true
                            label: qsTr("Frequency")
                            labelWidth: 56
                            valueWidth: 59
                            maximumTrackWidth: 96
                            from: 0
                            to: 1000
                            stepSize: 1
                            value: Math.log(Number(panel.selected.frequencyHertz) / 20) / Math.log(1000) * 1000
                            valueText: panel.formatFrequency(Math.round(20 * Math.pow(1000, value / 1000)))
                            onGestureStarted: panel.draft.beginGesture()
                            onGestureFinished: panel.draft.endGesture()
                            onEdited: sliderValue => panel.editSelected(Boolean(panel.selected.enabled), Number(panel.selected.filterKind), Math.round(20 * Math.pow(1000, sliderValue / 1000)), Number(panel.selected.qHundredths), Number(panel.selected.gainCentibels))
                        }

                        EchoParameterSlider {
                            Layout.fillWidth: true
                            label: qsTr("Q")
                            labelWidth: 56
                            valueWidth: 59
                            maximumTrackWidth: 96
                            from: 10
                            to: 2000
                            stepSize: 5
                            value: Number(panel.selected.qHundredths)
                            valueText: (value / 100).toFixed(2)
                            onGestureStarted: panel.draft.beginGesture()
                            onGestureFinished: panel.draft.endGesture()
                            onEdited: sliderValue => panel.editSelected(Boolean(panel.selected.enabled), Number(panel.selected.filterKind), Number(panel.selected.frequencyHertz), Math.round(sliderValue), Number(panel.selected.gainCentibels))
                        }

                        EchoParameterSlider {
                            Layout.fillWidth: true
                            label: qsTr("Gain")
                            labelWidth: 56
                            valueWidth: 59
                            maximumTrackWidth: 96
                            enabled: Number(panel.selected.filterKind) !== 3
                            from: -1200
                            to: 1200
                            stepSize: 10
                            value: Number(panel.selected.gainCentibels)
                            neutralValue: 0
                            showNeutralMarker: true
                            valueText: panel.formatGain(Math.round(value))
                            onGestureStarted: panel.draft.beginGesture()
                            onGestureFinished: panel.draft.endGesture()
                            onEdited: sliderValue => panel.editSelected(Boolean(panel.selected.enabled), Number(panel.selected.filterKind), Number(panel.selected.frequencyHertz), Number(panel.selected.qHundredths), Math.round(sliderValue))
                        }

                        Item {
                            Layout.fillHeight: true
                        }

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Drag a node to shape frequency and gain.")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            wrapMode: Text.WordWrap
                        }
                    }
                }
            }
        }
    }
}
