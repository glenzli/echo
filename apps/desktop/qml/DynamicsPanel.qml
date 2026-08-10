//! Compact stereo-linked compressor panel. The draft owns authored values;
//! this panel presents the transfer curve and precision controls only.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft

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

            Rectangle {
                implicitWidth: 30
                implicitHeight: 17
                radius: height / 2
                color: panel.draft.compressorEnabled
                    ? Theme.accent : Theme.track

                Rectangle {
                    width: 13
                    height: 13
                    radius: width / 2
                    y: 2
                    x: panel.draft.compressorEnabled ? parent.width - width - 2 : 2
                    color: panel.draft.compressorEnabled
                        ? Theme.accentText : Theme.panelRaised
                    Behavior on x { NumberAnimation { duration: 90 } }
                }

                TapHandler {
                    onTapped: panel.draft.setCompressorEnabled(
                        !panel.draft.compressorEnabled)
                }
            }

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

        Canvas {
            id: transferCurve

            Layout.fillWidth: true
            Layout.preferredHeight: 43
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 4
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
                    const y = height - coordinate(Math.max(-60, Math.min(0, output))) * height
                    if (input === -60) context.moveTo(x, y)
                    else context.lineTo(x, y)
                }
                context.stroke()
            }

            Connections {
                target: panel.draft
                function onCompressorEnabledChanged() { transferCurve.requestPaint() }
                function onCompressorThresholdCentibelsChanged() { transferCurve.requestPaint() }
                function onCompressorRatioTenthsChanged() { transferCurve.requestPaint() }
                function onCompressorMakeupCentibelsChanged() { transferCurve.requestPaint() }
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
            Layout.topMargin: 3
            Layout.bottomMargin: 3
            spacing: 0

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
}
