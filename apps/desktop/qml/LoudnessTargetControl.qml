//! Target-loudness selection and one-step clip-gain recommendation.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Item {
    id: control

    required property var draft
    required property var analyzer
    required property bool analysisCurrent

    property int targetLufs: -16

    readonly property var advice: analysisCurrent
        ? analyzer.gainAdvice(targetLufs,
            draft.limiterCeilingCentibels / 100,
            draft.gainCentibels)
        : ({ available: false, gainDeltaCentibels: 0,
             resultingGainCentibels: draft.gainCentibels,
             estimatedIntegratedLufs: -70,
             estimatedTruePeakDbtp: -70,
             peakConstrained: false, gainRangeConstrained: false,
             targetReached: false })
    readonly property bool constrained: advice.available
        && (advice.peakConstrained || advice.gainRangeConstrained)

    implicitHeight: 70

    function formatDelta(centibels: int) : string {
        const decibels = centibels / 100
        return (decibels > 0 ? "+" : "") + decibels.toFixed(1) + " dB"
    }

    function adviceSummary() : string {
        if (!analysisCurrent) return qsTr("Analyze to calculate gain")
        if (!advice.available) return qsTr("No gain advice for silence")
        return formatDelta(advice.gainDeltaCentibels) + "  →  "
            + Number(advice.estimatedIntegratedLufs).toFixed(1) + " LUFS"
    }

    function constraintSummary() : string {
        if (!advice.available) return ""
        if (advice.gainRangeConstrained) return qsTr("Clip gain range limits target")
        if (advice.peakConstrained) return qsTr("True-peak headroom limits target")
        if (advice.targetReached) return qsTr("Target is reachable transparently")
        return qsTr("Reanalyze after applying")
    }

    function applyAdvice() : void {
        if (!advice.available || advice.gainDeltaCentibels === 0) return
        draft.setGain(Number(advice.resultingGainCentibels))
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 8
        spacing: 2

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 27
            spacing: 5

            Text {
                text: qsTr("Target")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Item { Layout.fillWidth: true }

            Repeater {
                model: [-23, -18, -16, -14]

                delegate: Button {
                    id: targetChip

                    required property int modelData

                    Layout.preferredWidth: 34
                    Layout.preferredHeight: 21
                    padding: 0
                    text: modelData.toString()
                    checkable: true
                    checked: control.targetLufs === modelData
                    autoExclusive: true
                    onClicked: control.targetLufs = modelData

                    background: Rectangle {
                        radius: Theme.compactControlRadius
                        color: targetChip.checked
                            ? Theme.accentSurface
                            : targetChip.hovered ? Theme.buttonGhostHover
                                                 : Theme.controlQuiet
                        border.width: 1
                        border.color: targetChip.checked
                            ? Theme.accent : Theme.border
                    }

                    contentItem: Text {
                        text: targetChip.modelData
                        color: targetChip.checked
                            ? Theme.accentSelectionText : Theme.textSecondary
                        font.family: "Menlo"
                        font.pixelSize: Theme.fontMeta
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }

            Text {
                text: "LUFS"
                color: Theme.textDisabled
                font.pixelSize: 8
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 7

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    Layout.fillWidth: true
                    text: control.adviceSummary()
                    color: control.constrained ? Theme.warningText : Theme.textPrimary
                    font.family: control.advice.available ? "Menlo" : ""
                    font.pixelSize: Theme.fontMeta
                    font.weight: control.advice.available ? Font.DemiBold : Font.Normal
                    elide: Text.ElideRight
                }

                Text {
                    visible: text.length > 0
                    Layout.fillWidth: true
                    text: control.constraintSummary()
                    color: control.constrained ? Theme.warningText : Theme.textDisabled
                    font.pixelSize: 8
                    elide: Text.ElideRight
                }
            }

            EchoButton {
                text: qsTr("Apply")
                ghost: true
                enabled: control.analysisCurrent && control.advice.available
                    && control.advice.gainDeltaCentibels !== 0
                implicitWidth: 54
                implicitHeight: 24
                onClicked: control.applyAdvice()
            }
        }
    }
}
