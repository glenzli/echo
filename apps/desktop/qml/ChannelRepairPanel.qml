//! Authored stereo-channel repair controls. The draft owns validation,
//! history, and persistence; this component owns the compact projection only.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: panel

    required property var draft

    implicitWidth: 580
    implicitHeight: 302
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    function toggleEnabled(): void {
        draft.beginGesture();
        draft.setChannelRepairEnabled(!draft.channelRepairEnabled);
        draft.endGesture();
    }

    function setBooleanParameter(name: string, value: bool): void {
        draft.beginGesture();
        draft.setChannelRepairParameter(name, value);
        draft.endGesture();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 30
            Layout.leftMargin: 10
            Layout.rightMargin: 7
            spacing: 7

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/tune.svg"
                size: 15
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Channel repair")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: qsTr("Enabled")
                color: panel.draft.channelRepairEnabled ? Theme.textSecondary : Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }

            Rectangle {
                implicitWidth: 30
                implicitHeight: 17
                radius: height / 2
                color: panel.draft.channelRepairEnabled ? Theme.accent : Theme.track

                Rectangle {
                    width: 13
                    height: 13
                    radius: 7
                    y: 2
                    x: panel.draft.channelRepairEnabled ? parent.width - width - 2 : 2
                    color: panel.draft.channelRepairEnabled ? Theme.accentText : Theme.panelRaised

                    Behavior on x {
                        NumberAnimation {
                            duration: 90
                        }
                    }
                }

                TapHandler {
                    onTapped: panel.toggleEnabled()
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/reset-all.svg"
                toolTipText: qsTr("Reset channel repair")
                enabled: panel.draft.channelRepairEnabled || panel.draft.channelRepairInvertLeft || panel.draft.channelRepairInvertRight || panel.draft.channelRepairSwapChannels || panel.draft.channelRepairMonoFoldDown || panel.draft.channelRepairBalancePercent !== 0
                buttonSize: 25
                iconSize: 14
                onClicked: panel.draft.resetChannelRepair()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 10
            Layout.bottomMargin: 9
            spacing: 8

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 8
                rowSpacing: 8

                ChannelToggle {
                    Layout.fillWidth: true
                    label: qsTr("Invert left polarity")
                    checked: panel.draft.channelRepairInvertLeft
                    onToggled: panel.setBooleanParameter("invertLeft", checked)
                }

                ChannelToggle {
                    Layout.fillWidth: true
                    label: qsTr("Invert right polarity")
                    checked: panel.draft.channelRepairInvertRight
                    onToggled: panel.setBooleanParameter("invertRight", checked)
                }

                ChannelToggle {
                    Layout.fillWidth: true
                    label: qsTr("Swap left and right")
                    checked: panel.draft.channelRepairSwapChannels
                    onToggled: panel.setBooleanParameter("swapChannels", checked)
                }

                ChannelToggle {
                    Layout.fillWidth: true
                    label: qsTr("Fold down to mono")
                    checked: panel.draft.channelRepairMonoFoldDown
                    onToggled: panel.setBooleanParameter("monoFoldDown", checked)
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                Layout.maximumWidth: Theme.editorControlTrackWidth + 152
                labelWidth: 74
                label: qsTr("Balance")
                from: -100
                to: 100
                stepSize: 1
                value: panel.draft.channelRepairBalancePercent
                valueText: value === 0 ? qsTr("Center") : (value < 0 ? qsTr("L %1").arg(Math.abs(Math.round(value))) : qsTr("R %1").arg(Math.round(value)))
                onGestureStarted: panel.draft.beginGesture()
                onEdited: value => panel.draft.setChannelRepairParameter("balance", value)
                onGestureFinished: panel.draft.endGesture()
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Repairs polarity and channel-placement problems without widening or adding spatial effects.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }
        }
    }

    component ChannelToggle: Rectangle {
        id: toggle

        required property string label
        required property bool checked
        signal toggled(bool checked)

        implicitHeight: 34
        radius: Theme.compactControlRadius
        color: checked ? Theme.accentSurfaceQuiet : Theme.controlQuiet
        border.width: 1
        border.color: checked ? Theme.accentBorder : Theme.border

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 8
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: toggle.label
                color: toggle.checked ? Theme.textPrimary : Theme.textSecondary
                font.pixelSize: Theme.fontSection
                elide: Text.ElideRight
            }

            Rectangle {
                implicitWidth: 28
                implicitHeight: 16
                radius: height / 2
                color: toggle.checked ? Theme.accent : Theme.track

                Rectangle {
                    width: 12
                    height: 12
                    radius: 6
                    y: 2
                    x: toggle.checked ? parent.width - width - 2 : 2
                    color: toggle.checked ? Theme.accentText : Theme.panelRaised

                    Behavior on x {
                        NumberAnimation {
                            duration: 90
                        }
                    }
                }
            }
        }

        TapHandler {
            onTapped: toggle.toggled(!toggle.checked)
        }
    }
}
