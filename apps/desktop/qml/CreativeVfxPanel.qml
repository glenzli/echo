//! Composition shell for the four independent Creative VFX families. Each
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

    readonly property var availableFamilies: {
        const choices = [];
        const titles = [qsTr("Scene"), qsTr("Delay"), qsTr("Modulation"), qsTr("Transform")];
        for (let kind = 8; kind <= 11; ++kind) {
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
        }
    }
}
