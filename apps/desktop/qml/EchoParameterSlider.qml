//! Compact, value-first parameter row for precision audio controls. The
//! control owns presentation and gesture boundaries; the adjustment draft
//! remains the sole owner of mutation and history.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: field

    property string label: ""
    property string accessibleName: label
    property alias value: slider.value
    property alias from: slider.from
    property alias to: slider.to
    property alias stepSize: slider.stepSize
    property string valueText: ""
    property int labelWidth: label.length > 0 ? 72 : 0
    property int valueWidth: 64
    property int maximumTrackWidth: Theme.editorControlTrackWidth
    property real neutralValue: from
    property bool showNeutralMarker: false
    property bool fillFromMinimum: false
    signal edited(real value)
    signal gestureStarted
    signal gestureFinished

    implicitHeight: 30

    RowLayout {
        anchors.fill: parent
        spacing: 9

        Text {
            visible: field.label.length > 0
            Layout.preferredWidth: field.labelWidth
            Layout.minimumWidth: field.labelWidth
            text: field.label
            color: field.enabled ? Theme.textSecondary : Theme.textDisabled
            font.pixelSize: Theme.fontSection
            horizontalAlignment: Text.AlignRight
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        Slider {
            id: slider

            Layout.fillWidth: true
            Layout.minimumWidth: 72
            Layout.preferredWidth: field.maximumTrackWidth
            Layout.maximumWidth: field.maximumTrackWidth
            implicitHeight: 26
            enabled: field.enabled
            snapMode: Slider.SnapAlways
            Accessible.name: field.accessibleName
            Accessible.description: field.valueText

            background: Item {
                x: slider.leftPadding
                y: Math.round((slider.height - height) / 2)
                width: slider.availableWidth
                height: 10

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    height: 4
                    radius: 2
                    color: slider.enabled ? Theme.track : Theme.border
                }

                Rectangle {
                    visible: field.fillFromMinimum
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: slider.visualPosition * parent.width
                    height: 4
                    radius: 2
                    color: slider.enabled ? Theme.accent : Theme.textDisabled
                }

                Rectangle {
                    visible: field.showNeutralMarker
                    x: Math.round(Math.max(0, Math.min(parent.width - width, (field.neutralValue - slider.from) / Math.max(0.0001, slider.to - slider.from) * parent.width - width / 2)))
                    anchors.verticalCenter: parent.verticalCenter
                    width: 1
                    height: 8
                    color: slider.enabled ? Theme.textSecondary : Theme.textDisabled
                }
            }

            handle: Rectangle {
                x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
                y: Math.round((slider.height - height) / 2)
                implicitWidth: slider.pressed ? 14 : 12
                implicitHeight: slider.pressed ? 14 : 12
                radius: width / 2
                color: slider.enabled ? Theme.panelRaised : Theme.controlQuiet
                border.width: slider.pressed ? 2 : 1
                border.color: slider.enabled ? Theme.accent : Theme.textDisabled

                Behavior on implicitWidth {
                    NumberAnimation {
                        duration: 80
                    }
                }
                Behavior on implicitHeight {
                    NumberAnimation {
                        duration: 80
                    }
                }
            }

            onPressedChanged: {
                if (pressed)
                    field.gestureStarted();
                else
                    field.gestureFinished();
            }
            onMoved: field.edited(value)
        }

        Text {
            Layout.preferredWidth: field.valueWidth
            Layout.minimumWidth: field.valueWidth
            text: field.valueText
            color: field.enabled ? Theme.textPrimary : Theme.textDisabled
            font.family: "Menlo"
            font.pixelSize: Theme.fontSection
            font.weight: Font.Medium
            horizontalAlignment: Text.AlignRight
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideLeft
        }
    }
}
