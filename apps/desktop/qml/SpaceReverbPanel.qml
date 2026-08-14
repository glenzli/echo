//! Compact authored controls for Echo's algorithmic space characters.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft
    property var impulseResponses: impulseResponseController.impulseResponses

    implicitWidth: 560
    implicitHeight: 318
    radius: Theme.panelRadius
    color: Theme.parameterPanel
    border.width: 1
    border.color: Theme.border

    function frequency(hertz: int): string {
        return hertz >= 1000 ? (hertz / 1000).toFixed(hertz % 1000 === 0 ? 0 : 1) + " kHz" : hertz + " Hz";
    }

    function refreshImpulseResponses(): void {
        impulseResponseController.refresh();
    }

    function selectedImpulseIndex(): int {
        const selected = String(draft.space.impulseResponseImportId || "");
        for (let index = 0; index < impulseResponses.length; ++index) {
            if (String(impulseResponses[index].importId) === selected)
                return index;
        }
        return -1;
    }

    function selectedImpulseLayout(): string {
        const index = selectedImpulseIndex();
        return index >= 0 ? String(impulseResponses[index].layoutKind || "") : "";
    }

    function activateSpaceMode(index: int): void {
        if (index === 0) {
            draft.setSpaceMode(0);
            return;
        }
        if (String(draft.space.impulseResponseImportId || "").length > 0) {
            draft.setSpaceMode(1);
            return;
        }
        if (impulseResponses.length > 0) {
            draft.selectImpulseResponse(impulseResponses[0]);
            return;
        }
        importDialog.present();
    }

    Component.onCompleted: refreshImpulseResponses()

    Connections {
        target: impulseResponseController
        function onImported(value): void {
            panel.draft.selectImpulseResponse(value);
        }
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
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Space")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Rectangle {
                implicitWidth: statusText.implicitWidth + 12
                implicitHeight: 18
                radius: 9
                color: panel.draft.reverbEnabled ? Theme.surfaceSelected : Theme.surfaceSubtle

                Text {
                    id: statusText
                    anchors.centerIn: parent
                    text: panel.draft.reverbEnabled ? qsTr("LIVE") : qsTr("BYPASS")
                    color: panel.draft.reverbEnabled ? Theme.accent : Theme.textDisabled
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.5
                }
            }

            Item {
                Layout.fillWidth: true
            }

            EchoSegmentedControl {
                objectName: "spaceModeSelector"
                Layout.preferredWidth: 176
                Layout.preferredHeight: 28
                model: [qsTr("Algorithmic"), qsTr("Convolution")]
                currentIndex: Number(panel.draft.space.mode)
                onActivated: index => panel.activateSpaceMode(index)
            }

            EchoSegmentedControl {
                objectName: "spaceCharacterSelector"
                Layout.preferredWidth: 248
                Layout.preferredHeight: 28
                visible: Number(panel.draft.space.mode) === 0
                model: [qsTr("Room"), qsTr("Hall"), qsTr("Plate"), qsTr("Spring")]
                currentIndex: panel.draft.reverbCharacter
                onActivated: index => panel.draft.setReverbCharacter(index)
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset space")
                enabled: panel.draft.reverbCharacter !== 0 || panel.draft.reverbEnabled || panel.draft.reverbMixPercent !== 18 || panel.draft.reverbPreDelayMillis !== 20 || panel.draft.reverbDecayMillis !== 1800 || panel.draft.reverbSizePercent !== 55 || panel.draft.reverbDampingPercent !== 45 || panel.draft.reverbLowCutHertz !== 120 || panel.draft.reverbHighCutHertz !== 10000
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetReverb()
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
            Layout.bottomMargin: 7
            spacing: 18
            visible: Number(panel.draft.space.mode) === 0

            ColumnLayout {
                Layout.preferredWidth: Math.min(340, (panel.width - 58) / 2)
                Layout.minimumWidth: 236
                Layout.maximumWidth: 360
                Layout.fillHeight: true
                spacing: 1

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Mix")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: panel.draft.reverbMixPercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("mix", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Pre-delay")
                    from: 0
                    to: 200
                    stepSize: 1
                    value: panel.draft.reverbPreDelayMillis
                    valueText: Math.round(value) + " ms"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("preDelay", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Decay")
                    from: 100
                    to: 12000
                    stepSize: 50
                    value: panel.draft.reverbDecayMillis
                    valueText: (value / 1000).toFixed(2) + " s"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("decay", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Size")
                    from: 10
                    to: 100
                    stepSize: 1
                    value: panel.draft.reverbSizePercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("size", value)
                    onGestureFinished: panel.draft.endGesture()
                }
            }

            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                color: Theme.border
            }

            ColumnLayout {
                Layout.preferredWidth: Math.min(340, (panel.width - 58) / 2)
                Layout.minimumWidth: 236
                Layout.maximumWidth: 360
                Layout.fillHeight: true
                spacing: 1

                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Damping")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: panel.draft.reverbDampingPercent
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("damping", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("Low cut")
                    from: 20
                    to: 1000
                    stepSize: 10
                    value: panel.draft.reverbLowCutHertz
                    valueText: panel.frequency(value)
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("lowCut", value)
                    onGestureFinished: panel.draft.endGesture()
                }
                EchoParameterSlider {
                    Layout.fillWidth: true
                    label: qsTr("High cut")
                    from: 1000
                    to: 20000
                    stepSize: 100
                    value: panel.draft.reverbHighCutHertz
                    valueText: panel.frequency(value)
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setReverbParameter("highCut", value)
                    onGestureFinished: panel.draft.endGesture()
                }

                Item {
                    Layout.fillHeight: true
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Insert effect · before fade and master")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    horizontalAlignment: Text.AlignRight
                }
            }

            Item {
                Layout.fillWidth: true
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 14
            Layout.rightMargin: 14
            Layout.topMargin: 10
            Layout.bottomMargin: 10
            spacing: 8
            visible: Number(panel.draft.space.mode) === 1

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                ComboBox {
                    id: impulseSelector
                    Layout.fillWidth: true
                    model: panel.impulseResponses
                    textRole: "displayName"
                    currentIndex: panel.selectedImpulseIndex()
                    displayText: currentIndex >= 0 ? currentText : qsTr("Choose an imported impulse response")
                    onActivated: index => panel.draft.selectImpulseResponse(panel.impulseResponses[index])
                }

                Button {
                    text: qsTr("Import WAV…")
                    onClicked: importDialog.present()
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 18

                EchoParameterSlider {
                    Layout.preferredWidth: 330
                    label: qsTr("Mix")
                    from: 0
                    to: 100
                    stepSize: 1
                    value: Number(panel.draft.space.convolutionMixPercent)
                    valueText: Math.round(value) + "%"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setConvolutionParameter("mix", value)
                    onGestureFinished: panel.draft.endGesture()
                }

                EchoParameterSlider {
                    Layout.preferredWidth: 330
                    label: qsTr("Wet gain")
                    from: -2400
                    to: 1200
                    stepSize: 10
                    value: Number(panel.draft.space.convolutionWetGainCentibels)
                    valueText: (value / 100).toFixed(1) + " dB"
                    onGestureStarted: panel.draft.beginGesture()
                    onEdited: value => panel.draft.setConvolutionParameter("wetGain", value)
                    onGestureFinished: panel.draft.endGesture()
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Stereo IRs use parallel L→L and R→R processing. Echo does not label this true stereo.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            Text {
                Layout.fillWidth: true
                visible: impulseSelector.currentIndex >= 0
                text: {
                    if (impulseSelector.currentIndex < 0)
                        return "";
                    const value = panel.impulseResponses[impulseSelector.currentIndex];
                    const rights = value.rightsKind === "spdx" ? value.spdxExpression : qsTr("User-owned · no redistribution");
                    const layout = value.layoutKind === "true_stereo_ll_lr_rl_rr" ? qsTr("True stereo · LL / LR / RL / RR") : value.layoutKind === "stereo_parallel" ? qsTr("Stereo parallel") : qsTr("Mono");
                    return [layout, value.creator, rights, value.attribution].filter(part => String(part || "").length > 0).join(" · ");
                }
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }

    ImpulseResponseImportDialog {
        id: importDialog
    }
}
