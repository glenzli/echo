//! Clip-level processing that always precedes authored insert effects. This
//! panel is selected from the pinned source node in SoundSignalChain.

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

    implicitWidth: 800
    implicitHeight: 310
    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function formatDuration(millis: int): string {
        const safe = Math.max(0, millis);
        const minutes = Math.floor(safe / 60000);
        const seconds = Math.floor((safe % 60000) / 1000);
        const tenths = Math.floor((safe % 1000) / 100);
        return minutes + ":" + String(seconds).padStart(2, "0") + "." + tenths;
    }

    function formatGain(centibels: int): string {
        const decibels = centibels / 100;
        return (decibels >= 0 ? "+" : "") + decibels.toFixed(1) + " dB";
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        AdjustmentPanelHeader {
            Layout.fillWidth: true
            title: qsTr("Clip / preamp")
            iconSource: "qrc:/EchoDesktop/icons/crop.svg"
        }

        PanelDivider {}

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            Layout.topMargin: 12
            Layout.bottomMargin: 12
            spacing: Theme.editorPanelGap + 6

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 220
                spacing: 7

                SectionHeader {
                    Layout.fillWidth: true
                    title: qsTr("Clip")
                    iconSource: "qrc:/EchoDesktop/icons/crop.svg"

                    EchoIconButton {
                        visible: panel.hasTimeSelection
                        source: "qrc:/EchoDesktop/icons/crop.svg"
                        toolTipText: qsTr("Crop to selection")
                        enabled: panel.selectionEndMillis > panel.selectionStartMillis
                        buttonSize: 25
                        iconSize: 14
                        onClicked: panel.draft.setTrimRange(panel.selectionStartMillis, panel.selectionEndMillis)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 43
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
                }

                PanelDivider {}

                SectionHeader {
                    Layout.fillWidth: true
                    title: qsTr("Fades")
                    iconSource: "qrc:/EchoDesktop/icons/fade-smooth.svg"
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 5

                    FadeCurveRow {
                        Layout.fillWidth: true
                        label: qsTr("In")
                        durationText: panel.formatDuration(panel.draft.fadeInMillis)
                        curve: panel.draft.fadeInCurve
                        onCurveRequested: value => panel.draft.setFadeCurves(value, panel.draft.fadeOutCurve)
                    }

                    FadeCurveRow {
                        Layout.fillWidth: true
                        label: qsTr("Out")
                        durationText: panel.formatDuration(panel.draft.fadeOutMillis)
                        curve: panel.draft.fadeOutCurve
                        onCurveRequested: value => panel.draft.setFadeCurves(panel.draft.fadeInCurve, value)
                    }
                }

                Item {
                    Layout.fillHeight: true
                }
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 220
                spacing: 7

                SectionHeader {
                    Layout.fillWidth: true
                    title: qsTr("Low cut")
                    iconSource: "qrc:/EchoDesktop/icons/high-pass.svg"

                    EchoIconButton {
                        source: panel.draft.lowCutHertz > 0 ? "qrc:/EchoDesktop/icons/filter.svg" : "qrc:/EchoDesktop/icons/filter-off.svg"
                        toolTipText: panel.draft.lowCutHertz > 0 ? qsTr("Disable low cut") : qsTr("Enable low cut")
                        selected: panel.draft.lowCutHertz > 0
                        buttonSize: 25
                        iconSize: 14
                        onClicked: panel.draft.setLowCut(panel.draft.lowCutHertz === 0 ? 80 : 0)
                    }
                }

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Frequency")
                    from: 20
                    to: 240
                    stepSize: 1
                    value: panel.draft.lowCutHertz > 0 ? panel.draft.lowCutHertz : 80
                    valueText: panel.draft.lowCutHertz === 0 ? qsTr("Bypass") : panel.draft.lowCutHertz + " Hz"
                    accessibleName: qsTr("Low cut")
                    valueWidth: 64
                    fillFromMinimum: true
                    enabled: panel.draft.lowCutHertz > 0
                    onGestureStarted: panel.draft.beginGesture()
                    onGestureFinished: panel.draft.endGesture()
                    onEdited: value => panel.draft.setLowCut(value)
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Removes rumble before the insert chain.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }

                PanelDivider {}

                SectionHeader {
                    Layout.fillWidth: true
                    title: qsTr("Clip gain")
                    iconSource: "qrc:/EchoDesktop/icons/gain.svg"
                }

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Gain")
                    from: -2400
                    to: 1200
                    stepSize: 10
                    value: panel.draft.gainCentibels
                    valueText: panel.formatGain(panel.draft.gainCentibels)
                    accessibleName: qsTr("Clip gain")
                    valueWidth: 70
                    neutralValue: 0
                    showNeutralMarker: true
                    onGestureStarted: panel.draft.beginGesture()
                    onGestureFinished: panel.draft.endGesture()
                    onEdited: value => panel.draft.setGain(value)
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Sets level before restoration and effects.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }

                Item {
                    Layout.fillHeight: true
                }
            }
        }
    }

    component SectionHeader: RowLayout {
        id: section

        property string title: ""
        property url iconSource
        default property alias actions: actionRow.data

        spacing: 6

        EchoIcon {
            source: section.iconSource
            size: 14
            color: Theme.textSecondary
        }

        Text {
            text: section.title
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
            font.weight: Font.DemiBold
        }

        Item {
            Layout.fillWidth: true
        }

        RowLayout {
            id: actionRow
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
            font.pixelSize: Theme.fontBody
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

        spacing: 5

        Text {
            Layout.preferredWidth: 24
            text: curveControl.label
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSection
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignRight
        }

        Text {
            Layout.preferredWidth: 52
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

        Item {
            Layout.fillWidth: true
        }
    }
}
