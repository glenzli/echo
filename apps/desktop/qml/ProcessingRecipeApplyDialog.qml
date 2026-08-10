//! Controlled apply surface for one or many sounds. Recipe identity, merge
//! mode, and target identities remain caller-owned; this dialog only emits
//! selection and apply intent.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: dialog

    property var recipeModel: []
    property int modelRevision: 0
    property string selectedRecipeId: ""
    property string mergeMode: "merge"
    property var targetIds: []
    property string targetLabel: ""

    signal recipeSelected(string recipeId)
    signal mergeModeSelected(string mergeMode)
    signal applyRequested(string recipeId, string mergeMode, var targetIds)

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(650, parent.width - 40)
    height: Math.min(650, parent.height - 40)
    padding: 0
    modal: true
    dim: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    readonly property int targetCount: targetIds && targetIds.length !== undefined ? targetIds.length : 0

    function present(): void {
        open();
    }

    function targetSummary(): string {
        if (dialog.targetLabel.length > 0)
            return dialog.targetLabel;
        if (dialog.targetCount === 1)
            return qsTr("Current sound");
        return qsTr("%1 sounds selected").arg(dialog.targetCount);
    }

    background: Rectangle {
        radius: Theme.panelRadius
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 18
            spacing: 10

            Rectangle {
                Layout.preferredWidth: 36
                Layout.preferredHeight: 36
                radius: 9
                color: Theme.accentSurfaceQuiet

                EchoIcon {
                    anchors.centerIn: parent
                    source: "qrc:/EchoDesktop/icons/equalizer.svg"
                    size: 18
                    color: Theme.accentSelectionText
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("Apply processing recipe")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: dialog.targetSummary()
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            EchoButton {
                text: qsTr("Close")
                ghost: true
                onClicked: dialog.close()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: Math.max(0, dialog.width - 36)
                spacing: 13

                ProcessingRecipePicker {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 310
                    recipeModel: dialog.recipeModel
                    modelRevision: dialog.modelRevision
                    selectedRecipeId: dialog.selectedRecipeId
                    onRecipeSelected: recipeId => dialog.recipeSelected(recipeId)
                }

                EchoSectionLabel {
                    Layout.fillWidth: true
                    text: qsTr("How to apply")
                    hint: qsTr("The original recording always remains unchanged.")
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    ItemDelegate {
                        id: mergeChoice

                        Layout.fillWidth: true
                        Layout.preferredHeight: 58
                        padding: 0
                        checked: dialog.mergeMode === "merge"
                        Accessible.name: qsTr("Merge with current processing")
                        onClicked: dialog.mergeModeSelected("merge")

                        background: Rectangle {
                            radius: Theme.controlRadius
                            color: mergeChoice.checked ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle
                            border.width: 1
                            border.color: mergeChoice.checked ? Theme.accent : Theme.border
                        }

                        contentItem: RowLayout {
                            spacing: 10

                            RadioButton {
                                checked: mergeChoice.checked
                                onClicked: dialog.mergeModeSelected("merge")
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("Merge with current processing")
                                    color: Theme.textPrimary
                                    font.pixelSize: Theme.fontBody
                                    font.weight: Font.DemiBold
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("Update only the modules included in the recipe.")
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    wrapMode: Text.WordWrap
                                }
                            }
                        }
                    }

                    ItemDelegate {
                        id: replaceChoice

                        Layout.fillWidth: true
                        Layout.preferredHeight: 58
                        padding: 0
                        checked: dialog.mergeMode === "replace"
                        Accessible.name: qsTr("Replace current processing")
                        onClicked: dialog.mergeModeSelected("replace")

                        background: Rectangle {
                            radius: Theme.controlRadius
                            color: replaceChoice.checked ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle
                            border.width: 1
                            border.color: replaceChoice.checked ? Theme.accent : Theme.border
                        }

                        contentItem: RowLayout {
                            spacing: 10

                            RadioButton {
                                checked: replaceChoice.checked
                                onClicked: dialog.mergeModeSelected("replace")
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("Replace current processing")
                                    color: Theme.textPrimary
                                    font.pixelSize: Theme.fontBody
                                    font.weight: Font.DemiBold
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("Rebuild the processing chain from this recipe.")
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    wrapMode: Text.WordWrap
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: preserveText.implicitHeight + 20
                    radius: Theme.controlRadius
                    color: Theme.surfaceSubtle

                    Text {
                        id: preserveText

                        anchors.fill: parent
                        anchors.margins: 10
                        text: qsTr("Each sound keeps its own trim, fades, and clip gain in both modes.")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WordWrap
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                Item {
                    Layout.preferredHeight: 2
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 14
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: dialog.targetSummary()
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            EchoButton {
                text: qsTr("Cancel")
                ghost: true
                onClicked: dialog.close()
            }

            EchoButton {
                text: dialog.targetCount > 1 ? qsTr("Apply to %1 sounds").arg(dialog.targetCount) : qsTr("Apply to current sound")
                enabled: dialog.selectedRecipeId.length > 0 && dialog.targetCount > 0 && (dialog.mergeMode === "merge" || dialog.mergeMode === "replace")
                onClicked: {
                    dialog.applyRequested(dialog.selectedRecipeId, dialog.mergeMode, dialog.targetIds);
                    dialog.close();
                }
            }
        }
    }
}
