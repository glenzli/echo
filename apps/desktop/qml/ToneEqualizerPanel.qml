//! Real three-band restoration equalizer panel. Authored gain remains in the
//! adjustment draft; this component owns only response presentation and input.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft

    implicitWidth: 320
    implicitHeight: 224
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    function formatGain(centibels: int) : string {
        const decibels = centibels / 100
        return (decibels >= 0 ? "+" : "") + decibels.toFixed(1) + " dB"
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 30
            Layout.leftMargin: 10
            Layout.rightMargin: 7
            spacing: 6

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/equalizer.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Three-band EQ")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Item { Layout.fillWidth: true }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset equalizer")
                enabled: panel.draft.eqLowGainCentibels !== 0
                    || panel.draft.eqMidGainCentibels !== 0
                    || panel.draft.eqHighGainCentibels !== 0
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetEqualizer()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 78
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 6

            Canvas {
                id: responseCurve
                anchors.fill: parent

                function gainY(centibels) {
                    return height / 2 - centibels / 1200 * (height / 2 - 8)
                }

                onPaint: {
                    const context = getContext("2d")
                    context.clearRect(0, 0, width, height)
                    context.lineWidth = 1
                    context.strokeStyle = Theme.border
                    for (let row = 1; row < 4; ++row) {
                        const y = row * height / 4
                        context.beginPath()
                        context.moveTo(0, y)
                        context.lineTo(width, y)
                        context.stroke()
                    }
                    for (let column = 1; column < 4; ++column) {
                        const x = column * width / 4
                        context.beginPath()
                        context.moveTo(x, 0)
                        context.lineTo(x, height)
                        context.stroke()
                    }

                    const lowY = gainY(panel.draft.eqLowGainCentibels)
                    const midY = gainY(panel.draft.eqMidGainCentibels)
                    const highY = gainY(panel.draft.eqHighGainCentibels)
                    context.lineWidth = 2
                    context.strokeStyle = Theme.accent
                    context.beginPath()
                    context.moveTo(0, lowY)
                    context.bezierCurveTo(width * 0.18, lowY,
                        width * 0.32, midY, width * 0.5, midY)
                    context.bezierCurveTo(width * 0.68, midY,
                        width * 0.82, highY, width, highY)
                    context.stroke()
                }

                Connections {
                    target: panel.draft
                    function onEqLowGainCentibelsChanged() {
                        responseCurve.requestPaint()
                    }
                    function onEqMidGainCentibelsChanged() {
                        responseCurve.requestPaint()
                    }
                    function onEqHighGainCentibelsChanged() {
                        responseCurve.requestPaint()
                    }
                }
            }

            RowLayout {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom

                Text {
                    text: "120 Hz"
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: "1 kHz"
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: "8 kHz"
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                }
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
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 5
            Layout.bottomMargin: 6
            spacing: 2

            EqualizerRow {
                Layout.fillWidth: true
                label: qsTr("Low")
                value: panel.draft.eqLowGainCentibels
                onBandEdited: value => panel.draft.setEqualizerBand("low", value)
            }
            EqualizerRow {
                Layout.fillWidth: true
                label: qsTr("Mid")
                value: panel.draft.eqMidGainCentibels
                onBandEdited: value => panel.draft.setEqualizerBand("mid", value)
            }
            EqualizerRow {
                Layout.fillWidth: true
                label: qsTr("High")
                value: panel.draft.eqHighGainCentibels
                onBandEdited: value => panel.draft.setEqualizerBand("high", value)
            }
        }
    }

    component EqualizerRow: EchoParameterSlider {
        signal bandEdited(int value)

        from: -1200
        to: 1200
        stepSize: 10
        labelWidth: 38
        valueWidth: 66
        neutralValue: 0
        showNeutralMarker: true
        valueText: panel.formatGain(value)
        accessibleName: label
        onGestureStarted: panel.draft.beginGesture()
        onGestureFinished: panel.draft.endGesture()
        onEdited: sliderValue => bandEdited(Math.round(sliderValue))
    }
}
