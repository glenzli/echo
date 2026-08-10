//! Adjustment workbench shell. It owns draft-level actions and composes
//! independently evolving adjustment panels without taking over their controls.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: inspector

    required property var draft
    property bool hasTimeSelection: false
    property int selectionStartMillis: 0
    property int selectionEndMillis: 0

    implicitHeight: 300
    color: Theme.panel
    border.color: Theme.border

    function formatDuration(millis: int) : string {
        const safe = Math.max(0, millis)
        const minutes = Math.floor(safe / 60000)
        const seconds = Math.floor((safe % 60000) / 1000)
        const tenths = Math.floor((safe % 1000) / 100)
        return minutes + ":" + String(seconds).padStart(2, "0") + "." + tenths
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

            Text {
                text: qsTr("Adjustments")
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
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            Layout.topMargin: 7
            Layout.bottomMargin: 7
            spacing: 10

            BasicAdjustmentPanel {
                Layout.preferredWidth: Math.min(400,
                    Math.max(370, inspector.width * 0.265))
                Layout.minimumWidth: 370
                Layout.maximumWidth: 410
                Layout.fillHeight: true
                draft: inspector.draft
                hasTimeSelection: inspector.hasTimeSelection
                selectionStartMillis: inspector.selectionStartMillis
                selectionEndMillis: inspector.selectionEndMillis
            }

            ToneEqualizerPanel {
                Layout.preferredWidth: 320
                Layout.minimumWidth: 286
                Layout.maximumWidth: 350
                Layout.fillHeight: true
                draft: inspector.draft
            }

            DynamicsPanel {
                Layout.preferredWidth: 330
                Layout.minimumWidth: 300
                Layout.maximumWidth: 350
                Layout.fillHeight: true
                draft: inspector.draft
            }

            Item { Layout.fillWidth: true }
        }
    }
}
