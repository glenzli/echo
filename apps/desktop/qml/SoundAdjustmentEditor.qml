//! Compact precision rack for the editor draft. SoundAdjustmentDraft owns
//! mutation, validation, history, and persistence semantics; this component
//! presents the exact controls that complement direct timeline gestures.

pragma ComponentBehavior: Bound

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

    implicitHeight: 122
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
            Layout.preferredHeight: 33
            Layout.leftMargin: 10
            Layout.rightMargin: 8
            spacing: 7

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/tune.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Basic adjustments")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontSection
                font.weight: Font.DemiBold
                font.letterSpacing: 0.35
            }

            Rectangle {
                visible: inspector.hasTimeSelection
                Layout.preferredWidth: rangeText.implicitWidth + 14
                Layout.preferredHeight: 22
                radius: Theme.compactControlRadius
                color: Theme.surfaceSubtle
                border.width: 1
                border.color: Theme.border

                Text {
                    id: rangeText
                    anchors.centerIn: parent
                    text: qsTr("Range") + "  "
                        + inspector.formatDuration(inspector.selectionStartMillis)
                        + " — " + inspector.formatDuration(inspector.selectionEndMillis)
                    color: Theme.textSecondary
                    font.family: "Menlo"
                    font.pixelSize: 9
                }
            }

            Item { Layout.fillWidth: true }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/refresh.svg"
                toolTipText: qsTr("Revert")
                enabled: inspector.draft.dirty
                buttonSize: 27
                iconSize: 15
                onClicked: inspector.draft.revert()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Clear")
                enabled: !inspector.draft.identity
                buttonSize: 27
                iconSize: 15
                onClicked: inspector.draft.clear()
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 17
                color: Theme.border
            }

            EchoButton {
                text: qsTr("Save version")
                enabled: inspector.draft.dirty
                implicitWidth: 84
                implicitHeight: 27
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
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 7
            Layout.bottomMargin: 8
            spacing: 10

            AdjustmentSection {
                Layout.preferredWidth: 216
                Layout.minimumWidth: 196
                title: qsTr("Clip")
                iconSource: "qrc:/EchoDesktop/icons/crop.svg"

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 9

                    MetricValue {
                        Layout.fillWidth: true
                        label: qsTr("In")
                        value: inspector.formatDuration(inspector.draft.trimStartMillis)
                    }
                    MetricValue {
                        Layout.fillWidth: true
                        label: qsTr("Out")
                        value: inspector.formatDuration(inspector.draft.trimEndMillis)
                    }
                    MetricValue {
                        Layout.fillWidth: true
                        label: qsTr("Duration")
                        value: inspector.formatDuration(
                            inspector.draft.selectedDurationMillis)
                    }

                    EchoIconButton {
                        visible: inspector.hasTimeSelection
                        source: "qrc:/EchoDesktop/icons/crop.svg"
                        toolTipText: qsTr("Crop to selection")
                        enabled: inspector.selectionEndMillis
                            > inspector.selectionStartMillis
                        buttonSize: 26
                        iconSize: 15
                        onClicked: inspector.draft.setTrimRange(
                            inspector.selectionStartMillis,
                            inspector.selectionEndMillis)
                    }
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.preferredWidth: 232
                Layout.minimumWidth: 216
                title: qsTr("Fades")
                iconSource: "qrc:/EchoDesktop/icons/fade-smooth.svg"

                FadeCurveRow {
                    Layout.fillWidth: true
                    label: qsTr("In")
                    durationText: inspector.formatDuration(
                        inspector.draft.fadeInMillis)
                    curve: inspector.draft.fadeInCurve
                    onCurveRequested: value => inspector.draft.setFadeCurves(
                        value, inspector.draft.fadeOutCurve)
                }

                FadeCurveRow {
                    Layout.fillWidth: true
                    label: qsTr("Out")
                    durationText: inspector.formatDuration(
                        inspector.draft.fadeOutMillis)
                    curve: inspector.draft.fadeOutCurve
                    onCurveRequested: value => inspector.draft.setFadeCurves(
                        inspector.draft.fadeInCurve, value)
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.fillWidth: true
                Layout.minimumWidth: 176
                title: qsTr("Low cut")
                iconSource: "qrc:/EchoDesktop/icons/high-pass.svg"

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 4

                    EchoIconButton {
                        source: inspector.draft.lowCutHertz > 0
                            ? "qrc:/EchoDesktop/icons/filter.svg"
                            : "qrc:/EchoDesktop/icons/filter-off.svg"
                        toolTipText: inspector.draft.lowCutHertz > 0
                            ? qsTr("Disable low cut") : qsTr("Enable low cut")
                        selected: inspector.draft.lowCutHertz > 0
                        buttonSize: 26
                        iconSize: 15
                        onClicked: inspector.draft.setLowCut(
                            inspector.draft.lowCutHertz === 0 ? 80 : 0)
                    }

                    EchoParameterSlider {
                        Layout.fillWidth: true
                        from: 20
                        to: 240
                        stepSize: 1
                        value: inspector.draft.lowCutHertz > 0
                            ? inspector.draft.lowCutHertz : 80
                        valueText: inspector.draft.lowCutHertz === 0
                            ? qsTr("Bypass")
                            : inspector.draft.lowCutHertz + " Hz"
                        accessibleName: qsTr("Low cut")
                        valueWidth: 52
                        fillFromMinimum: true
                        enabled: inspector.draft.lowCutHertz > 0
                        onGestureStarted: inspector.draft.beginGesture()
                        onGestureFinished: inspector.draft.endGesture()
                        onEdited: value => inspector.draft.setLowCut(value)
                    }
                }
            }

            SectionDivider {}

            AdjustmentSection {
                Layout.fillWidth: true
                Layout.minimumWidth: 176
                title: qsTr("Clip gain")
                iconSource: "qrc:/EchoDesktop/icons/gain.svg"

                EchoParameterSlider {
                    Layout.fillWidth: true
                    from: -2400
                    to: 1200
                    stepSize: 10
                    value: inspector.draft.gainCentibels
                    valueText: inspector.formatGain(
                        inspector.draft.gainCentibels)
                    accessibleName: qsTr("Clip gain")
                    valueWidth: 64
                    neutralValue: 0
                    showNeutralMarker: true
                    onGestureStarted: inspector.draft.beginGesture()
                    onGestureFinished: inspector.draft.endGesture()
                    onEdited: value => inspector.draft.setGain(value)
                }
            }
        }
    }

    component AdjustmentSection: ColumnLayout {
        property string title: ""
        property url iconSource
        default property alias content: sectionContent.data

        Layout.fillHeight: true
        spacing: 5

        RowLayout {
            Layout.fillWidth: true
            spacing: 5

            EchoIcon {
                source: parent.parent.iconSource
                size: 13
                color: Theme.textSecondary
            }
            Text {
                text: parent.parent.title.toUpperCase()
                color: Theme.textSecondary
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.55
            }
            Item { Layout.fillWidth: true }
        }

        ColumnLayout {
            id: sectionContent
            Layout.fillWidth: true
            spacing: 3
        }

        Item { Layout.fillHeight: true }
    }

    component SectionDivider: Rectangle {
        Layout.preferredWidth: 1
        Layout.fillHeight: true
        Layout.topMargin: 1
        Layout.bottomMargin: 1
        color: Theme.border
    }

    component MetricValue: ColumnLayout {
        property string label: ""
        property string value: ""

        spacing: 2

        Text {
            text: parent.label.toUpperCase()
            color: Theme.textDisabled
            font.pixelSize: 8
            font.weight: Font.Medium
        }
        Text {
            text: parent.value
            color: Theme.textPrimary
            font.family: "Menlo"
            font.pixelSize: Theme.fontMeta
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
            Layout.preferredWidth: 18
            text: curveControl.label.toUpperCase()
            color: Theme.textSecondary
            font.pixelSize: 9
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignRight
        }

        Text {
            Layout.preferredWidth: 47
            text: curveControl.durationText
            color: Theme.textPrimary
            font.family: "Menlo"
            font.pixelSize: 9
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
                buttonSize: 24
                iconSize: 14
                onClicked: curveControl.curveRequested(index)
            }
        }

        Item { Layout.fillWidth: true }
    }
}
