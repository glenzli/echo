//! Typed chorus, flanger, phaser, and tremolo projection.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("modulation")
    readonly property int character: Number(family.character || 0)
    readonly property string modeName: ["chorus", "flanger", "phaser", "tremolo"][character]
    readonly property var parameters: family[modeName] || ({})

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateCharacter(value: int): void {
        const next = draft.creativeVfxFamily("modulation");
        next.character = value;
        draft.setCreativeVfxFamily("modulation", next);
    }

    function updateParameter(name: string, value: int): void {
        const next = draft.creativeVfxFamily("modulation");
        const parameters = draft.copyCreativeVfx(next[modeName]);
        parameters[name] = value;
        next[modeName] = parameters;
        draft.setCreativeVfxFamily("modulation", next);
    }

    function rate(value: real): string { return (value / 1000).toFixed(value < 1000 ? 2 : 1) + " Hz"; }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        RowLayout {
            Layout.fillWidth: true; Layout.preferredHeight: Theme.editorPanelHeaderHeight
            Layout.leftMargin: 12; Layout.rightMargin: 8; spacing: 7
            EchoIcon { source: "qrc:/EchoDesktop/icons/waveform.svg"; size: 15; color: Theme.textSecondary }
            Text { text: qsTr("Modulation VFX"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Item { Layout.fillWidth: true }
            EchoSwitch { checked: Boolean(panel.family.enabled); accessibleName: qsTr("Modulation VFX"); onToggled: panel.draft.setCreativeVfxFamilyEnabled("modulation", checked) }
        }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.margins: 14; spacing: 8
            EchoSegmentedControl {
                Layout.fillWidth: true; Layout.maximumWidth: 460
                model: [qsTr("Chorus"), qsTr("Flanger"), qsTr("Phaser"), qsTr("Tremolo")]
                currentIndex: panel.character
                onActivated: index => panel.updateCharacter(index)
            }
            GridLayout {
                Layout.fillWidth: true; Layout.maximumWidth: 760
                columns: 2; columnSpacing: 24; rowSpacing: 2
                EchoParameterSlider {
                    visible: panel.character !== 3; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Mix"); from: 0; to: 100; stepSize: 1
                    value: Number(panel.parameters.mixPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true; label: qsTr("Rate"); from: panel.character === 3 ? 100 : 50; to: panel.character === 0 ? 5000 : panel.character === 3 ? 20000 : 10000; stepSize: 10
                    value: Number(panel.parameters.rateMillihertz || 0); valueText: panel.rate(value)
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("rateMillihertz", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.character === 0 || panel.character === 1; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Base delay"); from: panel.character === 0 ? 5000 : 100; to: panel.character === 0 ? 25000 : 5000; stepSize: 100
                    value: Number(panel.parameters.minimumDelayMicroseconds || 0); valueText: (value / 1000).toFixed(1) + " ms"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("minimumDelayMicroseconds", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.character === 0 || panel.character === 1; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Sweep"); from: panel.character === 0 ? 500 : 100; to: panel.character === 0 ? 20000 : 10000; stepSize: 100
                    value: Number(panel.parameters.sweepMicroseconds || 0); valueText: (value / 1000).toFixed(1) + " ms"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("sweepMicroseconds", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.character === 2; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Sweep low"); from: 20; to: Math.max(20, Number(panel.parameters.sweepHighHertz || 20000) - 1); stepSize: 10
                    value: Number(panel.parameters.sweepLowHertz || 0); valueText: Math.round(value) + " Hz"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("sweepLowHertz", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.character === 2; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Sweep high"); from: Math.min(19999, Number(panel.parameters.sweepLowHertz || 20) + 1); to: 20000; stepSize: 10
                    value: Number(panel.parameters.sweepHighHertz || 0); valueText: value >= 1000 ? (value / 1000).toFixed(1) + " kHz" : Math.round(value) + " Hz"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("sweepHighHertz", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.character === 1 || panel.character === 2; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Feedback"); from: -90; to: 90; stepSize: 1; neutralValue: 0; showNeutralMarker: true
                    value: Number(panel.parameters.feedbackPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("feedbackPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.character === 3; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Depth"); from: 0; to: 100; stepSize: 1
                    value: Number(panel.parameters.depthPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("depthPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true; label: qsTr("Stereo phase"); labelWidth: 88; from: 0; to: 180; stepSize: 1
                    value: Number(panel.parameters.stereoPhaseDegrees || 0); valueText: Math.round(value) + "°"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("stereoPhaseDegrees", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
            }
            Item { Layout.fillHeight: true }
        }
    }
}
