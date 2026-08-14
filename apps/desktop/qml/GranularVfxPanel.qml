//! Deterministic granular texture controls. The saved seed keeps repeatable
//! scheduling; history and voices remain execution details of the audio core.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("granular")

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("granular");
        next[name] = value;
        draft.setCreativeVfxFamily("granular", next);
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
                text: qsTr("Granular")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }
            Text {
                text: qsTr("Deterministic texture")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }
            Item {
                Layout.fillWidth: true
            }
            EchoSwitch {
                objectName: "granularVfxEnabled"
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Granular")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("granular", checked)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            columns: 2
            columnSpacing: 24
            rowSpacing: 12

            EchoParameterSlider {
                Layout.preferredWidth: 300
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
                Layout.preferredWidth: 300
                label: qsTr("Grain size")
                from: 20
                to: 250
                stepSize: 1
                value: Number(panel.family.grainMillis || 20)
                valueText: Math.round(value) + " ms"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("grainMillis", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                Layout.preferredWidth: 300
                label: qsTr("Density")
                from: 10
                to: 400
                stepSize: 1
                value: Number(panel.family.densityTenthsHertz || 10)
                valueText: (value / 10).toFixed(1) + " Hz"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("densityTenthsHertz", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                Layout.preferredWidth: 300
                label: qsTr("Lookback")
                from: 0
                to: 1500
                stepSize: 5
                value: Number(panel.family.lookbackMillis || 0)
                valueText: Math.round(value) + " ms"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("lookbackMillis", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                Layout.preferredWidth: 300
                label: qsTr("Scatter")
                from: 0
                to: 750
                stepSize: 5
                value: Number(panel.family.scatterMillis || 0)
                valueText: Math.round(value) + " ms"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("scatterMillis", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                Layout.preferredWidth: 300
                label: qsTr("Pitch")
                from: -1200
                to: 1200
                stepSize: 10
                value: Number(panel.family.pitchCents || 0)
                valueText: (value / 100).toFixed(1) + " st"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("pitchCents", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }
            EchoParameterSlider {
                Layout.preferredWidth: 300
                label: qsTr("Stereo spread")
                from: 0
                to: 100
                stepSize: 1
                value: Number(panel.family.stereoSpreadPercent || 0)
                valueText: Math.round(value) + "%"
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.updateValue("stereoSpreadPercent", Math.round(value))
                onGestureFinished: panel.draft.endGesture()
            }
            Text {
                Layout.preferredWidth: 300
                text: qsTr("The seed is kept with the adjustment so the texture remains repeatable across preview and export.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }
        }
    }
}
