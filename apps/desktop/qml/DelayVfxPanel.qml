//! Creative slapback and echo controls backed by the typed Delay VFX family.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("delay")
    readonly property bool echoMode: Number(family.character || 0) === 1
    readonly property var parameters: echoMode ? family.echo || ({}) : family.slapback || ({})

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateHeader(name: string, value: var): void {
        const next = draft.creativeVfxFamily("delay");
        next[name] = value;
        draft.setCreativeVfxFamily("delay", next);
    }

    function updateParameter(name: string, value: int): void {
        const next = draft.creativeVfxFamily("delay");
        const mode = echoMode ? "echo" : "slapback";
        const parameters = draft.copyCreativeVfx(next[mode]);
        parameters[name] = value;
        next[mode] = parameters;
        draft.setCreativeVfxFamily("delay", next);
    }

    function frequency(value: real): string {
        return value >= 1000 ? (value / 1000).toFixed(value % 1000 === 0 ? 0 : 1) + " kHz" : Math.round(value) + " Hz";
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true; Layout.preferredHeight: Theme.editorPanelHeaderHeight
            Layout.leftMargin: 12; Layout.rightMargin: 8; spacing: 7
            EchoIcon { source: "qrc:/EchoDesktop/icons/history.svg"; size: 15; color: Theme.textSecondary }
            Text { text: qsTr("Delay VFX"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Item { Layout.fillWidth: true }
            EchoSwitch { checked: Boolean(panel.family.enabled); accessibleName: qsTr("Delay VFX"); onToggled: panel.draft.setCreativeVfxFamilyEnabled("delay", checked) }
        }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.margins: 14; spacing: 9
            EchoSegmentedControl {
                Layout.preferredWidth: 260
                model: [qsTr("Slapback"), qsTr("Echo")]
                currentIndex: Number(panel.family.character || 0)
                onActivated: index => panel.updateHeader("character", index)
            }
            GridLayout {
                Layout.fillWidth: true; Layout.maximumWidth: 720
                columns: 2; columnSpacing: 24; rowSpacing: 4
                EchoParameterSlider {
                    Layout.fillWidth: true; label: qsTr("Delay")
                    from: panel.echoMode ? 80 : 30; to: panel.echoMode ? 2000 : 180; stepSize: 1
                    value: Number(panel.parameters.delayMillis || 0); valueText: Math.round(value) + " ms"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("delayMillis", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true; label: qsTr("Mix")
                    from: 0; to: 100; stepSize: 1
                    value: Number(panel.parameters.mixPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("mixPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true; label: qsTr("High cut")
                    from: 1000; to: 20000; stepSize: 100
                    value: Number(panel.parameters.highCutHertz || 0); valueText: panel.frequency(value)
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("highCutHertz", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.echoMode; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Feedback")
                    from: 0; to: 90; stepSize: 1
                    value: Number(panel.parameters.feedbackPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("feedbackPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    visible: panel.echoMode; enabled: visible
                    Layout.fillWidth: true; label: qsTr("Crossfeed")
                    from: 0; to: 100; stepSize: 1
                    value: Number(panel.parameters.stereoCrossfeedPercent || 0); valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture(); onEdited: value => panel.updateParameter("stereoCrossfeedPercent", Math.round(value)); onGestureFinished: panel.draft.endGesture()
                }
            }
            Item { Layout.fillHeight: true }
        }
    }
}
