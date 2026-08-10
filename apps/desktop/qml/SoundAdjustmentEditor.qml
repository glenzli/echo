//! Bottom precision console for the editor draft. SoundAdjustmentDraft owns
//! mutation, validation, history, and persistence semantics; this component
//! arranges exact controls below the direct timeline gestures.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: inspector

    required property var draft
    property bool hasTimeSelection: false
    property int selectionStartMillis: 0
    property int selectionEndMillis: 0

    implicitHeight: 220
    color: Theme.panel
    border.color: Theme.border

    function formatDuration(millis: int) : string {
        const safe = Math.max(0, millis)
        const minutes = Math.floor(safe / 60000)
        const seconds = Math.floor((safe % 60000) / 1000)
        const tenths = Math.floor((safe % 1000) / 100)
        return minutes + ":" + String(seconds).padStart(2, "0") + "." + tenths
    }

    function formatGain(centibels: int) : string {
        const decibels = centibels / 100
        return (decibels >= 0 ? "+" : "") + decibels.toFixed(1) + " dB"
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 1
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 38
            Layout.leftMargin: 12
            Layout.rightMargin: 10
            spacing: 8

            Text {
                text: qsTr("Adjustments")
                color: Theme.textPrimary
                font.pixelSize: 14
                font.weight: Font.DemiBold
            }

            Rectangle {
                visible: inspector.hasTimeSelection
                Layout.preferredWidth: 1
                Layout.preferredHeight: 15
                color: Theme.border
            }

            Text {
                visible: inspector.hasTimeSelection
                text: qsTr("Range") + " "
                    + inspector.formatDuration(inspector.selectionStartMillis)
                    + " — " + inspector.formatDuration(inspector.selectionEndMillis)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Revert")
                ghost: true
                enabled: inspector.draft.dirty
                onClicked: inspector.draft.revert()
            }

            EchoButton {
                text: qsTr("Clear")
                ghost: true
                enabled: !inspector.draft.identity
                onClicked: inspector.draft.clear()
            }

            EchoButton {
                text: qsTr("Save version")
                enabled: inspector.draft.dirty
                onClicked: inspector.draft.save()
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
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 9
            Layout.bottomMargin: 10
            spacing: 12

            AdjustmentSection {
                Layout.preferredWidth: 184
                title: qsTr("Clip")

                ColumnLayout {
                    width: parent.width
                    spacing: 6

                    InspectorValueRow {
                        label: qsTr("In")
                        value: inspector.formatDuration(inspector.draft.trimStartMillis)
                    }
                    InspectorValueRow {
                        label: qsTr("Out")
                        value: inspector.formatDuration(inspector.draft.trimEndMillis)
                    }
                    InspectorValueRow {
                        label: qsTr("Duration")
                        value: inspector.formatDuration(inspector.draft.selectedDurationMillis)
                    }

                    EchoButton {
                        Layout.fillWidth: true
                        visible: inspector.hasTimeSelection
                        text: qsTr("Crop to selection")
                        enabled: inspector.selectionEndMillis
                            > inspector.selectionStartMillis
                        onClicked: inspector.draft.setTrimRange(
                            inspector.selectionStartMillis,
                            inspector.selectionEndMillis)
                    }
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.fillWidth: true
                title: qsTr("Fade in")

                FadeCurveControl {
                    width: parent.width
                    durationText: inspector.formatDuration(inspector.draft.fadeInMillis)
                    curve: inspector.draft.fadeInCurve
                    onCurveRequested: value => inspector.draft.setFadeCurves(
                        value, inspector.draft.fadeOutCurve)
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.fillWidth: true
                title: qsTr("Fade out")

                FadeCurveControl {
                    width: parent.width
                    durationText: inspector.formatDuration(inspector.draft.fadeOutMillis)
                    curve: inspector.draft.fadeOutCurve
                    onCurveRequested: value => inspector.draft.setFadeCurves(
                        inspector.draft.fadeInCurve, value)
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.fillWidth: true
                title: qsTr("Low cut")

                ColumnLayout {
                    width: parent.width
                    spacing: 6

                    RowLayout {
                        Layout.fillWidth: true

                        EchoButton {
                            text: inspector.draft.lowCutHertz === 0
                                ? qsTr("Off") : qsTr("On")
                            ghost: true
                            selected: inspector.draft.lowCutHertz > 0
                            onClicked: inspector.draft.setLowCut(
                                inspector.draft.lowCutHertz === 0 ? 80 : 0)
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: inspector.draft.lowCutHertz === 0
                                ? qsTr("Bypass")
                                : inspector.draft.lowCutHertz + " Hz"
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                        }
                    }

                    Slider {
                        Layout.fillWidth: true
                        from: 20
                        to: 240
                        stepSize: 1
                        value: inspector.draft.lowCutHertz > 0
                            ? inspector.draft.lowCutHertz : 80
                        enabled: inspector.draft.lowCutHertz > 0
                        Accessible.name: qsTr("Low cut frequency")
                        onPressedChanged: {
                            if (pressed) inspector.draft.beginGesture()
                            else inspector.draft.endGesture()
                        }
                        onMoved: inspector.draft.setLowCut(value)
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "20 Hz"
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "240 Hz"
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                    }
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.fillWidth: true
                title: qsTr("Clip gain")

                ColumnLayout {
                    width: parent.width
                    spacing: 6

                    RowLayout {
                        Layout.fillWidth: true

                        Text {
                            text: qsTr("Output")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: inspector.formatGain(inspector.draft.gainCentibels)
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                        }
                    }

                    Slider {
                        Layout.fillWidth: true
                        from: -2400
                        to: 1200
                        stepSize: 10
                        value: inspector.draft.gainCentibels
                        Accessible.name: qsTr("Clip gain")
                        onPressedChanged: {
                            if (pressed) inspector.draft.beginGesture()
                            else inspector.draft.endGesture()
                        }
                        onMoved: inspector.draft.setGain(value)
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Text {
                            text: "−24"
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "0 dB"
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Item { Layout.fillWidth: true }
                        Text {
                            text: "+12"
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                    }
                }
            }
        }
    }

    component AdjustmentSection: ColumnLayout {
        property string title: ""
        default property alias content: sectionContent.data

        Layout.fillHeight: true
        spacing: 8

        Text {
            text: parent.title
            color: Theme.textPrimary
            font.pixelSize: Theme.fontSection
            font.weight: Font.DemiBold
        }

        ColumnLayout {
            id: sectionContent
            Layout.fillWidth: true
            spacing: 7
        }

        Item { Layout.fillHeight: true }
    }

    component SectionDivider: Rectangle {
        Layout.preferredWidth: 1
        Layout.fillHeight: true
        Layout.topMargin: 4
        Layout.bottomMargin: 4
        color: Theme.border
    }

    component InspectorValueRow: RowLayout {
        property string label: ""
        property string value: ""

        spacing: 8

        Text {
            text: parent.label
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
        }
        Item { Layout.fillWidth: true }
        Text {
            text: parent.value
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
            font.weight: Font.Medium
        }
    }

    component FadeCurveControl: ColumnLayout {
        id: curveControl

        property int curve: 0
        property string durationText: ""
        signal curveRequested(int value)

        spacing: 7

        RowLayout {
            Layout.fillWidth: true
            Text {
                text: qsTr("Length")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
            }
            Item { Layout.fillWidth: true }
            Text {
                text: curveControl.durationText
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 4

            Repeater {
                model: [qsTr("Linear"), qsTr("Smooth"), qsTr("Equal power")]

                delegate: EchoButton {
                    required property int index
                    required property string modelData

                    Layout.fillWidth: true
                    text: modelData
                    ghost: true
                    selected: curveControl.curve === index
                    onClicked: curveControl.curveRequested(index)
                }
            }
        }
    }
}
