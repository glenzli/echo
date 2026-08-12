//! Creative scene coloration without generated ambience or source mutation.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    readonly property var family: draft.creativeVfxFamily("scene")

    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function updateValue(name: string, value: var): void {
        const next = draft.creativeVfxFamily("scene");
        next[name] = value;
        draft.setCreativeVfxFamily("scene", next);
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

            EchoIcon { source: "qrc:/EchoDesktop/icons/filter.svg"; size: 15; color: Theme.textSecondary }
            Text { text: qsTr("Scene VFX"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold }
            Text { text: qsTr("Playback stays silent when the source is silent"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            EchoSwitch {
                checked: Boolean(panel.family.enabled)
                accessibleName: qsTr("Scene VFX")
                onToggled: panel.draft.setCreativeVfxFamilyEnabled("scene", checked)
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            spacing: 14

            EchoSegmentedControl {
                Layout.fillWidth: true
                Layout.maximumWidth: 560
                model: [qsTr("Telephone"), qsTr("Radio"), qsTr("Intercom"), qsTr("Behind wall"), qsTr("Underwater")]
                currentIndex: Number(panel.family.character || 0)
                onActivated: index => panel.updateValue("character", index)
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.maximumWidth: 640
                spacing: 20

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Mix")
                    from: 0; to: 100; stepSize: 1
                    value: Number(panel.family.mixPercent || 0)
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("mixPercent", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Intensity")
                    from: 0; to: 100; stepSize: 1
                    value: Number(panel.family.intensityPercent || 0)
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.updateValue("intensityPercent", Math.round(value))
                    onGestureFinished: panel.draft.endGesture()
                }
            }

            Item { Layout.fillHeight: true }
        }
    }
}
