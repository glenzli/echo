//! Composition shell for the independent Creative VFX families. Each
//! family keeps its own semantic panel; this owner only projects navigation.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Item {
    id: panel

    required property var draft
    property int familyKind: 8
    signal familySelected(int kind)

    readonly property var presetTitles: [qsTr("Voice memo"), qsTr("Night drive"), qsTr("Dream voice")]

    readonly property var availableFamilies: {
        const choices = [];
        const titles = [qsTr("Scene"), qsTr("Delay"), qsTr("Modulation"), qsTr("Transform"), qsTr("Degrade"), qsTr("Drive"), qsTr("Rotary"), qsTr("Freeze"), qsTr("Granular"), qsTr("Tape"), qsTr("Pitch"), qsTr("Auto-Wah")];
        for (let kind = 8; kind <= 19; ++kind) {
            if (draft.effectChain.indexOf(kind) >= 0) {
                choices.push({
                    kind: kind,
                    title: titles[kind - 8],
                    enabled: draft.effectNodeEnabled(kind)
                });
            }
        }
        return choices;
    }

    implicitWidth: 760
    implicitHeight: 342

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 30
            spacing: 5

            EchoIcon {
                source: "qrc:/EchoDesktop/icons/sparkles.svg"
                size: 14
                color: Theme.textSecondary
            }

            Text {
                text: qsTr("Creative")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 14
                color: Theme.border
            }

            Repeater {
                model: panel.availableFamilies

                delegate: Button {
                    id: familyButton

                    required property var modelData

                    implicitHeight: 26
                    leftPadding: 9
                    rightPadding: 9
                    topPadding: 0
                    bottomPadding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: panel.familySelected(Number(modelData.kind))

                    background: Rectangle {
                        radius: Theme.compactControlRadius
                        color: Number(familyButton.modelData.kind) === panel.familyKind ? Theme.surfaceSelected : familyButton.hovered ? Theme.buttonGhostHover : Theme.transparent
                        border.width: Number(familyButton.modelData.kind) === panel.familyKind ? 1 : 0
                        border.color: Theme.borderStrong
                    }

                    contentItem: RowLayout {
                        spacing: 6

                        Rectangle {
                            Layout.preferredWidth: 6
                            Layout.preferredHeight: 6
                            radius: 3
                            color: familyButton.modelData.enabled ? Theme.accent : Theme.textDisabled
                        }

                        Text {
                            text: String(familyButton.modelData.title)
                            color: Number(familyButton.modelData.kind) === panel.familyKind ? Theme.textPrimary : Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            font.weight: Number(familyButton.modelData.kind) === panel.familyKind ? Font.DemiBold : Font.Normal
                        }
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: qsTr("Stylized, non-destructive effects")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 26
            spacing: 8

            Text {
                text: qsTr("Scenes")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                font.weight: Font.DemiBold
            }

            Repeater {
                model: panel.presetTitles

                delegate: Button {
                    required property string modelData
                    implicitHeight: 24
                    leftPadding: 9
                    rightPadding: 9
                    topPadding: 0
                    bottomPadding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: panel.draft.applyCreativePreset(index)

                    background: Rectangle {
                        radius: Theme.compactControlRadius
                        color: parent.hovered ? Theme.buttonGhostHover : Theme.transparent
                        border.width: 1
                        border.color: Theme.border
                    }

                    contentItem: Text {
                        text: parent.modelData
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }

            Item { Layout.fillWidth: true }
            Text { text: qsTr("Applies an undoable effect recipe"); color: Theme.textDisabled; font.pixelSize: Theme.fontMeta }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            SceneVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 8
                draft: panel.draft
            }

            DelayVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 9
                draft: panel.draft
            }

            ModulationVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 10
                draft: panel.draft
            }

            TransformVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 11
                draft: panel.draft
            }

            DigitalDegradeVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 12
                draft: panel.draft
            }

            DriveVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 13
                draft: panel.draft
            }

            RotaryVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 14
                draft: panel.draft
            }

            FreezeVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 15
                draft: panel.draft
            }

            GranularVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 16
                draft: panel.draft
            }

            TapeVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 17
                draft: panel.draft
            }

            PitchVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 18
                draft: panel.draft
            }

            AutoWahVfxPanel {
                anchors.fill: parent
                visible: panel.familyKind === 19
                draft: panel.draft
            }
        }
    }
}
