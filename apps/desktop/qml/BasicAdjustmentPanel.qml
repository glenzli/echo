//! Cohesive presentation owner for Echo's implemented foundation controls.
//! Controls flow vertically so this panel stays narrow beside advanced tools.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    property bool hasTimeSelection: false
    property int selectionStartMillis: 0
    property int selectionEndMillis: 0

    implicitWidth: 390
    implicitHeight: 224
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

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
        spacing: 0

        PanelHeader {
            Layout.fillWidth: true
            title: qsTr("Basic adjustments")
            iconSource: "qrc:/EchoDesktop/icons/tune.svg"
        }

        PanelDivider {}

        ControlRow {
            Layout.fillWidth: true
            Layout.preferredHeight: 43
            title: qsTr("Clip")
            iconSource: "qrc:/EchoDesktop/icons/crop.svg"

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                MetricValue {
                    Layout.fillWidth: true
                    label: qsTr("In")
                    value: panel.formatDuration(panel.draft.trimStartMillis)
                }
                MetricValue {
                    Layout.fillWidth: true
                    label: qsTr("Out")
                    value: panel.formatDuration(panel.draft.trimEndMillis)
                }
                MetricValue {
                    Layout.fillWidth: true
                    label: qsTr("Duration")
                    value: panel.formatDuration(panel.draft.selectedDurationMillis)
                }

                EchoIconButton {
                    visible: panel.hasTimeSelection
                    source: "qrc:/EchoDesktop/icons/crop.svg"
                    toolTipText: qsTr("Crop to selection")
                    enabled: panel.selectionEndMillis > panel.selectionStartMillis
                    buttonSize: 27
                    iconSize: 15
                    onClicked: panel.draft.setTrimRange(
                        panel.selectionStartMillis, panel.selectionEndMillis)
                }
            }
        }

        PanelDivider {}

        ControlRow {
            Layout.fillWidth: true
            Layout.preferredHeight: 62
            title: qsTr("Fades")
            iconSource: "qrc:/EchoDesktop/icons/fade-smooth.svg"

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3

                FadeCurveRow {
                    Layout.fillWidth: true
                    label: qsTr("In")
                    durationText: panel.formatDuration(panel.draft.fadeInMillis)
                    curve: panel.draft.fadeInCurve
                    onCurveRequested: value => panel.draft.setFadeCurves(
                        value, panel.draft.fadeOutCurve)
                }

                FadeCurveRow {
                    Layout.fillWidth: true
                    label: qsTr("Out")
                    durationText: panel.formatDuration(panel.draft.fadeOutMillis)
                    curve: panel.draft.fadeOutCurve
                    onCurveRequested: value => panel.draft.setFadeCurves(
                        panel.draft.fadeInCurve, value)
                }
            }
        }

        PanelDivider {}

        ControlRow {
            Layout.fillWidth: true
            Layout.preferredHeight: 43
            title: qsTr("Low cut")
            iconSource: "qrc:/EchoDesktop/icons/high-pass.svg"

            RowLayout {
                Layout.fillWidth: true
                spacing: 5

                EchoIconButton {
                    source: panel.draft.lowCutHertz > 0
                        ? "qrc:/EchoDesktop/icons/filter.svg"
                        : "qrc:/EchoDesktop/icons/filter-off.svg"
                    toolTipText: panel.draft.lowCutHertz > 0
                        ? qsTr("Disable low cut") : qsTr("Enable low cut")
                    selected: panel.draft.lowCutHertz > 0
                    buttonSize: 27
                    iconSize: 15
                    onClicked: panel.draft.setLowCut(
                        panel.draft.lowCutHertz === 0 ? 80 : 0)
                }

                EchoParameterSlider {
                    Layout.fillWidth: true
                    from: 20
                    to: 240
                    stepSize: 1
                    value: panel.draft.lowCutHertz > 0
                        ? panel.draft.lowCutHertz : 80
                    valueText: panel.draft.lowCutHertz === 0
                        ? qsTr("Bypass") : panel.draft.lowCutHertz + " Hz"
                    accessibleName: qsTr("Low cut")
                    valueWidth: 56
                    fillFromMinimum: true
                    enabled: panel.draft.lowCutHertz > 0
                    onGestureStarted: panel.draft.beginGesture()
                    onGestureFinished: panel.draft.endGesture()
                    onEdited: value => panel.draft.setLowCut(value)
                }
            }
        }

        PanelDivider {}

        ControlRow {
            Layout.fillWidth: true
            Layout.fillHeight: true
            title: qsTr("Clip gain")
            iconSource: "qrc:/EchoDesktop/icons/gain.svg"

            EchoParameterSlider {
                Layout.fillWidth: true
                from: -2400
                to: 1200
                stepSize: 10
                value: panel.draft.gainCentibels
                valueText: panel.formatGain(panel.draft.gainCentibels)
                accessibleName: qsTr("Clip gain")
                valueWidth: 66
                neutralValue: 0
                showNeutralMarker: true
                onGestureStarted: panel.draft.beginGesture()
                onGestureFinished: panel.draft.endGesture()
                onEdited: value => panel.draft.setGain(value)
            }
        }
    }

    component PanelHeader: RowLayout {
        property string title: ""
        property url iconSource

        Layout.preferredHeight: 30
        Layout.leftMargin: 10
        Layout.rightMargin: 8
        spacing: 6

        EchoIcon {
            source: parent.iconSource
            size: 14
            color: Theme.textSecondary
        }
        Text {
            text: parent.title
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
            font.weight: Font.DemiBold
        }
        Item { Layout.fillWidth: true }
    }

    component ControlRow: RowLayout {
        property string title: ""
        property url iconSource
        default property alias content: rowContent.data

        Layout.leftMargin: 10
        Layout.rightMargin: 10
        Layout.topMargin: 4
        Layout.bottomMargin: 4
        spacing: 8

        RowLayout {
            Layout.preferredWidth: 68
            Layout.minimumWidth: 68
            spacing: 5

            EchoIcon {
                source: parent.parent.iconSource
                size: 13
                color: Theme.textSecondary
            }
            Text {
                Layout.fillWidth: true
                text: parent.parent.title
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSection
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
        }

        RowLayout {
            id: rowContent
            Layout.fillWidth: true
            spacing: 4
        }
    }

    component PanelDivider: Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 1
        color: Theme.border
    }

    component MetricValue: ColumnLayout {
        property string label: ""
        property string value: ""

        spacing: 1

        Text {
            text: parent.label
            color: Theme.textDisabled
            font.pixelSize: Theme.fontMeta
            font.weight: Font.Medium
        }
        Text {
            text: parent.value
            color: Theme.textPrimary
            font.family: "Menlo"
            font.pixelSize: Theme.fontSection
            font.weight: Font.Medium
            elide: Text.ElideRight
        }
    }

    component FadeCurveRow: RowLayout {
        id: curveControl

        property int curve: 0
        property string label: ""
        property string durationText: ""
        signal curveRequested(int value)

        spacing: 4

        Text {
            Layout.preferredWidth: 22
            text: curveControl.label
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSection
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignRight
        }

        Text {
            Layout.preferredWidth: 50
            text: curveControl.durationText
            color: Theme.textPrimary
            font.family: "Menlo"
            font.pixelSize: Theme.fontSection
            horizontalAlignment: Text.AlignRight
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 17
            color: Theme.border
        }

        Repeater {
            model: [
                {
                    "name": qsTr("Linear"),
                    "icon": "qrc:/EchoDesktop/icons/fade-linear.svg"
                },
                {
                    "name": qsTr("Smooth"),
                    "icon": "qrc:/EchoDesktop/icons/fade-smooth.svg"
                },
                {
                    "name": qsTr("Equal power"),
                    "icon": "qrc:/EchoDesktop/icons/fade-equal-power.svg"
                }
            ]

            delegate: EchoIconButton {
                required property int index
                required property var modelData

                source: modelData.icon
                toolTipText: modelData.name
                selected: curveControl.curve === index
                buttonSize: 25
                iconSize: 14
                onClicked: curveControl.curveRequested(index)
            }
        }

        Item { Layout.fillWidth: true }
    }
}
