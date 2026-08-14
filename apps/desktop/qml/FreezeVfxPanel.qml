//! Source-anchored spectral hold. The processor owns FFT state and pre-roll;
//! this panel persists only the intentional capture point and wet amount.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("freeze")
    readonly property int safeCaptureMinimum: 86

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("freeze");
        next[name] = value;
        draft.setCreativeVfxFamily("freeze", next);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.editorPanelHeaderHeight
            Layout.leftMargin: 12
            Layout.rightMargin: 8
            spacing: 7

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/sparkles.svg"
                size: 15
                color: Theme.textSecondary
            }
            Text {
                text: qsTr("Freeze")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Text {
                text: qsTr("Spectral hold")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }
            Item {
                Layout.fillWidth: true
            }
            EchoSwitch {
                objectName: "freezeVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Freeze")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("freeze", checked)
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
            Layout.margins: 14
            spacing: 14

            EchoParameterSlider {
                objectName: "freezeVfxMix"
                Layout.preferredWidth: 360
                label: qsTr("Mix")
                from: 0
                to: 100
                stepSize: 1
                value: Number(panel.family.mixPercent || 0)
                valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("mixPercent", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }

            EchoParameterSlider {
                objectName: "freezeVfxCapture"
                Layout.preferredWidth: 520
                label: qsTr("Capture point")
                from: panel.safeCaptureMinimum
                to: Math.max(panel.safeCaptureMinimum, Number(panel.draft.trimEndMillis) - 1)
                stepSize: 1
                value: Math.max(panel.safeCaptureMinimum, Number(panel.family.captureSourceMillis || panel.safeCaptureMinimum))
                valueText: (value / 1000).toFixed(2) + " s"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("captureSourceMillis", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }

            Text {
                Layout.preferredWidth: 520
                text: qsTr("Holds a spectrum captured from the original source. Echo pre-rolls the required history; it does not generate new material.")
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
