//! Compact stereo-linked compressor panel. The draft owns authored values;
//! this panel presents the transfer curve and precision controls only.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    required property var meterSource

    implicitWidth: 330
    implicitHeight: 224
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    function decibels(centibels: int, signed: bool) : string {
        const value = centibels / 100
        return (signed && value > 0 ? "+" : "") + value.toFixed(1) + " dB"
    }

    function meterText(value: real, unit: string) : string {
        return value <= -69.9 ? "—" : value.toFixed(1) + " " + unit
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
                source: "qrc:/EchoDesktop/icons/dynamics.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Dynamics")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Item { Layout.fillWidth: true }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset compressor")
                enabled: panel.draft.compressorEnabled
                    || panel.draft.compressorThresholdCentibels !== -1800
                    || panel.draft.compressorRatioTenths !== 30
                    || panel.draft.compressorAttackMillis !== 10
                    || panel.draft.compressorReleaseMillis !== 120
                    || panel.draft.compressorMakeupCentibels !== 0
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetCompressor()
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
            spacing: 12

            ColumnLayout {
                Layout.preferredWidth: Math.min(290, panel.width * 0.4)
                Layout.minimumWidth: 210
                Layout.maximumWidth: 300
                Layout.fillHeight: true
                spacing: 7

                Canvas {
                    id: transferCurve

                    Layout.fillWidth: true
                    Layout.preferredHeight: 72
                    opacity: panel.draft.compressorEnabled ? 1 : 0.45

                    function outputLevel(input) {
                        const threshold = panel.draft.compressorThresholdCentibels / 100
                        const ratio = panel.draft.compressorRatioTenths / 10
                        const offset = input - threshold
                        if (offset > 3) return threshold + offset / ratio
                        if (offset <= -3) return input
                        const knee = offset + 3
                        return input + (1 / ratio - 1) * knee * knee / 12
                    }

                    function coordinate(level) {
                        return (level + 60) / 60
                    }

                    onPaint: {
                        const context = getContext("2d")
                        context.clearRect(0, 0, width, height)
                        context.strokeStyle = Theme.border
                        context.lineWidth = 1
                        context.beginPath()
                        context.moveTo(0, height)
                        context.lineTo(width, 0)
                        context.stroke()
                        context.strokeStyle = Theme.accent
                        context.lineWidth = 2
                        context.beginPath()
                        for (let input = -60; input <= 0; input += 1) {
                            const x = coordinate(input) * width
                            const output = outputLevel(input)
                                + panel.draft.compressorMakeupCentibels / 100
                            const y = height - coordinate(Math.max(-60,
                                Math.min(0, output))) * height
                            if (input === -60) context.moveTo(x, y)
                            else context.lineTo(x, y)
                        }
                        context.stroke()
                    }

                    Connections {
                        target: panel.draft
                        function onCompressorEnabledChanged() {
                            transferCurve.requestPaint()
                        }
                        function onCompressorThresholdCentibelsChanged() {
                            transferCurve.requestPaint()
                        }
                        function onCompressorRatioTenthsChanged() {
                            transferCurve.requestPaint()
                        }
                        function onCompressorMakeupCentibelsChanged() {
                            transferCurve.requestPaint()
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 38
                    spacing: 8

                    MeterReadout {
                        label: qsTr("Momentary")
                        value: panel.meterSource.momentaryLufs
                        minimum: -60
                        maximum: 0
                        valueText: panel.meterText(value, "LUFS")
                        meterColor: Theme.accent
                    }
                    MeterReadout {
                        label: qsTr("Peak")
                        value: panel.meterSource.outputPeakDb
                        minimum: -60
                        maximum: 0
                        valueText: panel.meterText(value, "dBFS")
                        meterColor: value > -1 ? Theme.warningText : Theme.accent
                    }
                    MeterReadout {
                        label: qsTr("Reduction")
                        value: panel.meterSource.gainReductionDb
                        minimum: 0
                        maximum: 24
                        valueText: panel.meterSource.active
                            ? value.toFixed(1) + " dB" : "—"
                        meterColor: Theme.accentSelectionText
                    }
                }

                Item { Layout.fillHeight: true }
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            ColumnLayout {
                Layout.preferredWidth: 370
                Layout.minimumWidth: 280
                Layout.maximumWidth: 420
                Layout.fillHeight: true
                spacing: 2

                DynamicsRow {
                    label: qsTr("Threshold")
                    from: -6000
                    to: 0
                    stepSize: 10
                    value: panel.draft.compressorThresholdCentibels
                    valueText: panel.decibels(value, false)
                    onParameterEdited: value => panel.draft.setCompressorParameter("threshold", value)
                }
                DynamicsRow {
                    label: qsTr("Ratio")
                    from: 10
                    to: 200
                    stepSize: 1
                    value: panel.draft.compressorRatioTenths
                    valueText: (value / 10).toFixed(1) + ":1"
                    onParameterEdited: value => panel.draft.setCompressorParameter("ratio", value)
                }
                DynamicsRow {
                    label: qsTr("Attack")
                    from: 1
                    to: 200
                    stepSize: 1
                    value: panel.draft.compressorAttackMillis
                    valueText: Math.round(value) + " ms"
                    onParameterEdited: value => panel.draft.setCompressorParameter("attack", value)
                }
                DynamicsRow {
                    label: qsTr("Release")
                    from: 20
                    to: 2000
                    stepSize: 10
                    value: panel.draft.compressorReleaseMillis
                    valueText: Math.round(value) + " ms"
                    onParameterEdited: value => panel.draft.setCompressorParameter("release", value)
                }
                DynamicsRow {
                    label: qsTr("Makeup")
                    from: 0
                    to: 2400
                    stepSize: 10
                    value: panel.draft.compressorMakeupCentibels
                    valueText: panel.decibels(value, true)
                    onParameterEdited: value => panel.draft.setCompressorParameter("makeup", value)
                }

                Item { Layout.fillHeight: true }
            }

            Item { Layout.fillWidth: true }
        }
    }

    component DynamicsRow: EchoParameterSlider {
        signal parameterEdited(int value)

        Layout.fillWidth: true
        implicitHeight: 25
        labelWidth: 58
        valueWidth: 64
        fillFromMinimum: true
        accessibleName: label
        onGestureStarted: panel.draft.beginGesture()
        onGestureFinished: panel.draft.endGesture()
        onEdited: sliderValue => parameterEdited(Math.round(sliderValue))
    }

    component MeterReadout: Item {
        id: readout

        property string label: ""
        property real value: 0
        property real minimum: 0
        property real maximum: 1
        property string valueText: ""
        property color meterColor: Theme.accent
        readonly property real meterProgress: Math.max(0, Math.min(1,
            (value - minimum) / Math.max(0.0001, maximum - minimum)))

        Layout.fillWidth: true
        Layout.minimumWidth: 72
        implicitHeight: 34

        Text {
            anchors.left: parent.left
            anchors.top: parent.top
            text: readout.label
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
        }

        Text {
            anchors.right: parent.right
            anchors.top: parent.top
            text: readout.valueText
            color: Theme.textPrimary
            font.family: "Menlo"
            font.pixelSize: Theme.fontMeta
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 4
            radius: 2
            color: Theme.track

            Rectangle {
                width: parent.width * readout.meterProgress
                height: parent.height
                radius: parent.radius
                color: readout.meterColor
                Behavior on width { NumberAnimation { duration: 70 } }
            }
        }
    }
}
