//! Precision inspector for the editor draft. SoundAdjustmentDraft owns all
//! mutation, validation, history, and persistence semantics; this component
//! provides the compact exact-control surface beside direct timeline gestures.

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

    implicitWidth: 296
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

    function curveName(value: int) : string {
        if (value === 1) return qsTr("Smooth")
        if (value === 2) return qsTr("Equal power")
        return qsTr("Linear")
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 1
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 52
            Layout.leftMargin: 16
            Layout.rightMargin: 12
            spacing: 8

            ColumnLayout {
                spacing: 2

                Text {
                    text: qsTr("Adjustments")
                    color: Theme.textPrimary
                    font.pixelSize: 14
                    font.weight: Font.DemiBold
                }

                Text {
                    text: inspector.draft.dirty ? qsTr("Unsaved changes") : qsTr("Saved version")
                    color: inspector.draft.dirty ? Theme.warningText : Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                }
            }

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Undo")
                ghost: true
                enabled: inspector.draft.canUndo
                onClicked: inspector.draft.undo()
            }

            EchoButton {
                text: qsTr("Redo")
                ghost: true
                enabled: inspector.draft.canRedo
                onClicked: inspector.draft.redo()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: parent.width
                spacing: 0

                InspectorSection {
                    title: qsTr("Time selection")
                    visible: inspector.hasTimeSelection
                    Layout.fillWidth: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 10

                        InspectorValueRow {
                            label: qsTr("Range")
                            value: inspector.formatDuration(inspector.selectionStartMillis)
                                + " — " + inspector.formatDuration(inspector.selectionEndMillis)
                        }

                        InspectorValueRow {
                            label: qsTr("Duration")
                            value: inspector.formatDuration(inspector.selectionEndMillis
                                - inspector.selectionStartMillis)
                        }

                        EchoButton {
                            Layout.fillWidth: true
                            text: qsTr("Crop to selection")
                            enabled: inspector.selectionEndMillis
                                > inspector.selectionStartMillis
                            onClicked: inspector.draft.setTrimRange(
                                inspector.selectionStartMillis,
                                inspector.selectionEndMillis)
                        }
                    }
                }

                InspectorSection {
                    title: qsTr("Clip")
                    Layout.fillWidth: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 8

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
                    }
                }

                InspectorSection {
                    title: qsTr("Fade in")
                    Layout.fillWidth: true

                    FadeCurveControl {
                        width: parent.width
                        durationText: inspector.formatDuration(inspector.draft.fadeInMillis)
                        curve: inspector.draft.fadeInCurve
                        onCurveRequested: value => inspector.draft.setFadeCurves(
                            value, inspector.draft.fadeOutCurve)
                    }
                }

                InspectorSection {
                    title: qsTr("Fade out")
                    Layout.fillWidth: true

                    FadeCurveControl {
                        width: parent.width
                        durationText: inspector.formatDuration(inspector.draft.fadeOutMillis)
                        curve: inspector.draft.fadeOutCurve
                        onCurveRequested: value => inspector.draft.setFadeCurves(
                            inspector.draft.fadeInCurve, value)
                    }
                }

                InspectorSection {
                    title: qsTr("Clip gain")
                    Layout.fillWidth: true

                    ColumnLayout {
                        width: parent.width
                        spacing: 8

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
                            id: gainSlider

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

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            spacing: 8

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

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Save version")
                enabled: inspector.draft.dirty
                onClicked: inspector.draft.save()
            }
        }
    }

    component InspectorSection: ColumnLayout {
        property string title: ""
        default property alias content: sectionContent.data

        spacing: 10
        Layout.leftMargin: 16
        Layout.rightMargin: 16
        Layout.topMargin: 14
        Layout.bottomMargin: 14

        Text {
            text: parent.title
            color: Theme.textPrimary
            font.pixelSize: Theme.fontSection
            font.weight: Font.DemiBold
        }

        ColumnLayout {
            id: sectionContent
            Layout.fillWidth: true
            spacing: 8
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: 4
            Layout.preferredHeight: 1
            color: Theme.border
        }
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

        spacing: 9

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
