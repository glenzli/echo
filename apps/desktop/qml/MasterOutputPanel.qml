//! Final output limiter and whole-preview loudness analysis presentation.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    required property var meterSource
    required property var analyzer
    required property string sourcePath
    required property string analysisKey

    readonly property bool analysisCurrent: analyzer.hasResult
        && analyzer.resultKey === analysisKey

    implicitWidth: 286
    implicitHeight: 286
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    function decibels(centibels: int) : string {
        return (centibels / 100).toFixed(1) + " dBTP"
    }

    function runAnalysis() : void {
        if (analyzer.running) {
            analyzer.cancel()
            return
        }
        analyzer.analyzeAdjusted(
            analysisKey, sourcePath,
            draft.trimStartMillis, draft.trimEndMillis,
            draft.fadeInMillis, draft.fadeOutMillis,
            draft.fadeInCurve, draft.fadeOutCurve,
            draft.gainCentibels, draft.lowCutHertz,
            draft.restorationValue(), draft.deHumValue(), draft.deClickValue(),
            draft.equalizerEnabled,
            draft.equalizerBands, draft.compressorEnabled,
            draft.compressorThresholdCentibels, draft.compressorRatioTenths,
            draft.compressorAttackMillis, draft.compressorReleaseMillis,
            draft.compressorMakeupCentibels, draft.reverbValue(), draft.limiterEnabled,
            draft.limiterCeilingCentibels, draft.limiterReleaseMillis,
            draft.effectChain)
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
                source: "qrc:/EchoDesktop/icons/gain.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Master")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Item { Layout.fillWidth: true }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset limiter")
                enabled: panel.draft.limiterEnabled
                    || panel.draft.limiterCeilingCentibels !== -100
                    || panel.draft.limiterReleaseMillis !== 100
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetLimiter()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 4
            Layout.bottomMargin: 3
            spacing: 0

            EchoParameterSlider {
                Layout.fillWidth: true
                implicitHeight: 27
                label: qsTr("Ceiling")
                from: -600
                to: 0
                stepSize: 10
                value: panel.draft.limiterCeilingCentibels
                valueText: panel.decibels(value)
                valueWidth: 74
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setLimiterParameter("ceiling", value)
                onGestureFinished: panel.draft.endGesture()
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                implicitHeight: 27
                label: qsTr("Release")
                from: 20
                to: 1000
                stepSize: 10
                value: panel.draft.limiterReleaseMillis
                valueText: Math.round(value) + " ms"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setLimiterParameter("release", value)
                onGestureFinished: panel.draft.endGesture()
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 26
                spacing: 8

                Text {
                    text: qsTr("Limiter reduction")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: panel.meterSource.active
                        ? panel.meterSource.limiterReductionDb.toFixed(1) + " dB" : "—"
                    color: Theme.textPrimary
                    font.family: "Menlo"
                    font.pixelSize: Theme.fontMeta
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 33
            Layout.leftMargin: 10
            Layout.rightMargin: 8
            spacing: 8

            Text {
                text: qsTr("Loudness analysis")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }
            Item { Layout.fillWidth: true }
            EchoButton {
                text: panel.analyzer.running ? qsTr("Cancel") : qsTr("Analyze")
                ghost: true
                implicitWidth: 66
                implicitHeight: 24
                onClicked: panel.runAnalysis()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 2
            color: Theme.track

            Rectangle {
                width: parent.width * (panel.analyzer.running ? panel.analyzer.progress
                    : panel.analysisCurrent ? 1 : 0)
                height: parent.height
                color: Theme.accent
                Behavior on width { NumberAnimation { duration: 100 } }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 54
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 4
            Layout.bottomMargin: 5
            spacing: 8

            Text {
                visible: panel.analyzer.errorText.length > 0
                Layout.fillWidth: true
                text: qsTr("Analysis failed")
                color: Theme.warningText
                font.pixelSize: Theme.fontMeta
                horizontalAlignment: Text.AlignHCenter
            }

            AnalysisValue {
                visible: panel.analyzer.errorText.length === 0
                label: qsTr("Integrated")
                value: panel.analysisCurrent
                    ? panel.analyzer.integratedLufs.toFixed(1) : "—"
                unit: "LUFS"
            }
            AnalysisValue {
                visible: panel.analyzer.errorText.length === 0
                label: qsTr("True peak estimate")
                value: panel.analysisCurrent
                    ? panel.analyzer.truePeakDbtp.toFixed(1) : "—"
                unit: "dBTP"
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        LoudnessTargetControl {
            Layout.fillWidth: true
            Layout.fillHeight: true
            draft: panel.draft
            analyzer: panel.analyzer
            analysisCurrent: panel.analysisCurrent
        }
    }

    component AnalysisValue: Rectangle {
        required property string label
        required property string value
        required property string unit

        Layout.fillWidth: true
        Layout.fillHeight: true
        radius: Theme.compactControlRadius
        color: Theme.surfaceSubtle

        Column {
            anchors.centerIn: parent
            spacing: 2

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: label
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: value
                color: Theme.textPrimary
                font.family: "Menlo"
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: unit
                color: Theme.textDisabled
                font.pixelSize: 8
            }
        }
    }
}
