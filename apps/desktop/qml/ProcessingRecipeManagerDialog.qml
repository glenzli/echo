//! Controlled management surface for named processing recipes. The caller
//! owns recipe identity, revisions, persistence and operation results; this
//! dialog owns only transient rename, update and archive confirmation forms.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: dialog

    // Entries accept { id, name, revisionId, revisionNumber, components,
    // updatedAtMillis }. User-authored names and sourceLabel are displayed
    // byte-for-byte and never translated.
    property var recipeModel: []
    property int modelRevision: 0
    property string selectedRecipeId: ""
    property string sourceAssetId: ""
    property string sourceLabel: ""
    property bool canUpdateFromSource: false

    signal recipeSelected(string recipeId)
    signal renameRequested(string recipeId, string name)
    signal updateRequested(string recipeId, var componentIds)
    signal archiveRequested(string recipeId)

    property bool renaming: false
    property bool choosingUpdate: false

    readonly property var selectedRecipe: {
        const observedRevision = dialog.modelRevision;
        const count = dialog.modelCount();
        for (let index = 0; index < count; ++index) {
            const entry = dialog.modelEntry(index);
            if (entry && String(entry.id || entry.recipeId || "") === dialog.selectedRecipeId)
                return entry;
        }
        return null;
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(760, parent.width - 40)
    height: Math.min(620, parent.height - 40)
    padding: 0
    modal: true
    dim: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function modelCount(): int {
        if (!dialog.recipeModel)
            return 0;
        if (dialog.recipeModel.count !== undefined)
            return dialog.recipeModel.count;
        return dialog.recipeModel.length !== undefined ? dialog.recipeModel.length : 0;
    }

    function modelEntry(index: int): var {
        if (dialog.recipeModel.get !== undefined)
            return dialog.recipeModel.get(index);
        return dialog.recipeModel[index];
    }

    function selectedRecipeName(): string {
        return dialog.selectedRecipe ? String(dialog.selectedRecipe.name || "") : "";
    }

    function componentTitle(componentId: string): string {
        if (componentId === "lowCut")
            return qsTr("Low cut");
        if (componentId === "restoration")
            return qsTr("Restoration");
        if (componentId === "deHum")
            return qsTr("De-hum");
        if (componentId === "deClick")
            return qsTr("De-click");
        if (componentId === "equalizer")
            return qsTr("Equalizer");
        if (componentId === "dynamics")
            return qsTr("Dynamics");
        if (componentId === "space")
            return qsTr("Space");
        if (componentId === "master")
            return qsTr("Master output");
        return componentId;
    }

    function recipeComponents(): var {
        if (!dialog.selectedRecipe || !dialog.selectedRecipe.components || dialog.selectedRecipe.components.length === undefined)
            return [];
        return dialog.selectedRecipe.components;
    }

    function selectedUpdateComponents(): var {
        const componentIds = [];
        for (let index = 0; index < updateComponents.count; ++index) {
            const entry = updateComponents.get(index);
            if (entry.included)
                componentIds.push(entry.componentId);
        }
        return componentIds;
    }

    function rebuildUpdateComponents(): void {
        const selected = ({});
        const current = dialog.recipeComponents();
        for (let index = 0; index < current.length; ++index)
            selected[String(current[index])] = true;
        const orderedIds = ["lowCut", "restoration", "deHum", "deClick", "equalizer", "dynamics", "space", "master"];
        updateComponents.clear();
        for (let index = 0; index < orderedIds.length; ++index) {
            const componentId = orderedIds[index];
            updateComponents.append({
                componentId: componentId,
                included: Boolean(selected[componentId])
            });
        }
    }

    function beginRename(): void {
        if (!dialog.selectedRecipe)
            return;
        dialog.choosingUpdate = false;
        dialog.renaming = true;
        renameField.text = dialog.selectedRecipeName();
        renameValidation.text = "";
        renameField.forceActiveFocus();
        renameField.selectAll();
    }

    function beginUpdate(): void {
        if (!dialog.selectedRecipe || !dialog.canUpdateFromSource || dialog.sourceAssetId.length === 0)
            return;
        dialog.renaming = false;
        dialog.choosingUpdate = true;
        updateValidation.text = "";
        dialog.rebuildUpdateComponents();
    }

    function present(): void {
        dialog.renaming = false;
        dialog.choosingUpdate = false;
        open();
    }

    onSelectedRecipeIdChanged: {
        dialog.renaming = false;
        dialog.choosingUpdate = false;
    }

    background: Rectangle {
        radius: Theme.panelRadius
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    ListModel {
        id: updateComponents
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 18
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("Manage processing recipes")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Rename, update, or archive reusable processing")
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

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 12
            spacing: 12

            ProcessingRecipePicker {
                Layout.preferredWidth: 310
                Layout.fillHeight: true
                recipeModel: dialog.recipeModel
                modelRevision: dialog.modelRevision
                selectedRecipeId: dialog.selectedRecipeId
                onRecipeSelected: recipeId => dialog.recipeSelected(recipeId)
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.fillHeight: true
                color: Theme.border
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

                ColumnLayout {
                    width: Math.max(0, parent.width - 14)
                    spacing: 14

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        visible: dialog.selectedRecipe !== null

                        Text {
                            Layout.fillWidth: true
                            text: dialog.selectedRecipeName()
                            color: Theme.textPrimary
                            font.pixelSize: 17
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: dialog.selectedRecipe ? qsTr("Version %1").arg(Number(dialog.selectedRecipe.revisionNumber || 1)) : ""
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                        }
                    }

                    EchoSectionLabel {
                        Layout.fillWidth: true
                        visible: dialog.selectedRecipe !== null
                        text: qsTr("Included processing")
                    }

                    Flow {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.max(25, childrenRect.height)
                        spacing: 5
                        visible: dialog.selectedRecipe !== null

                        Repeater {
                            model: dialog.recipeComponents()

                            delegate: Rectangle {
                                id: componentChip

                                required property var modelData

                                width: componentLabel.implicitWidth + 16
                                height: 25
                                radius: Theme.compactControlRadius
                                color: Theme.surfaceSubtle
                                border.width: 1
                                border.color: Theme.border

                                Text {
                                    id: componentLabel

                                    anchors.centerIn: parent
                                    text: dialog.componentTitle(String(componentChip.modelData))
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                }
                            }
                        }

                        Text {
                            visible: dialog.recipeComponents().length === 0
                            text: qsTr("No processing modules")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontBody
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: renameContent.implicitHeight + 20
                        visible: dialog.selectedRecipe !== null
                        radius: Theme.controlRadius
                        color: Theme.surfaceSubtle
                        border.width: 1
                        border.color: Theme.border

                        ColumnLayout {
                            id: renameContent

                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 10
                            spacing: 7

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                EchoSectionLabel {
                                    Layout.fillWidth: true
                                    text: qsTr("Name")
                                    hint: qsTr("The recipe name is yours and is never translated.")
                                }

                                EchoButton {
                                    visible: !dialog.renaming
                                    text: qsTr("Rename")
                                    ghost: true
                                    onClicked: dialog.beginRename()
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                visible: dialog.renaming
                                spacing: 7

                                EchoTextField {
                                    id: renameField

                                    Layout.fillWidth: true
                                    maximumLength: 80
                                    Accessible.name: qsTr("Processing recipe name")
                                    onAccepted: renameConfirm.clicked()
                                    onTextChanged: renameValidation.text = ""
                                }

                                EchoButton {
                                    text: qsTr("Cancel")
                                    ghost: true
                                    onClicked: dialog.renaming = false
                                }

                                EchoButton {
                                    id: renameConfirm

                                    text: qsTr("Save name")
                                    enabled: renameField.text.trim().length > 0
                                    onClicked: {
                                        if (renameField.text.trim().length === 0) {
                                            renameValidation.text = qsTr("Enter a processing recipe name.");
                                            return;
                                        }
                                        dialog.renameRequested(dialog.selectedRecipeId, renameField.text);
                                        dialog.renaming = false;
                                    }
                                }
                            }

                            Text {
                                id: renameValidation

                                Layout.fillWidth: true
                                visible: dialog.renaming && renameValidation.text.length > 0
                                color: Theme.warningText
                                font.pixelSize: Theme.fontMeta
                                wrapMode: Text.WordWrap
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: updateContent.implicitHeight + 20
                        visible: dialog.selectedRecipe !== null
                        radius: Theme.controlRadius
                        color: Theme.surfaceSubtle
                        border.width: 1
                        border.color: Theme.border

                        ColumnLayout {
                            id: updateContent

                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 10
                            spacing: 8

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 8

                                EchoSectionLabel {
                                    Layout.fillWidth: true
                                    text: qsTr("Update from saved sound")
                                    hint: dialog.sourceLabel.length > 0 ? dialog.sourceLabel : qsTr("No saved source sound selected")
                                }

                                EchoButton {
                                    visible: !dialog.choosingUpdate
                                    text: qsTr("Choose modules")
                                    ghost: true
                                    enabled: dialog.canUpdateFromSource && dialog.sourceAssetId.length > 0
                                    onClicked: dialog.beginUpdate()
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                visible: !dialog.canUpdateFromSource || dialog.sourceAssetId.length === 0
                                text: qsTr("Save the current sound before using it to update a recipe.")
                                color: Theme.textDisabled
                                font.pixelSize: Theme.fontMeta
                                wrapMode: Text.WordWrap
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                visible: dialog.choosingUpdate
                                spacing: 3

                                Repeater {
                                    model: updateComponents

                                    delegate: CheckDelegate {
                                        id: updateComponentRow

                                        required property string componentId
                                        required property bool included
                                        required property int index

                                        Layout.fillWidth: true
                                        implicitHeight: 34
                                        text: dialog.componentTitle(updateComponentRow.componentId)
                                        checked: updateComponentRow.included
                                        onToggled: updateComponents.setProperty(updateComponentRow.index, "included", checked)
                                    }
                                }

                                Text {
                                    id: updateValidation

                                    Layout.fillWidth: true
                                    visible: updateValidation.text.length > 0
                                    color: Theme.warningText
                                    font.pixelSize: Theme.fontMeta
                                    wrapMode: Text.WordWrap
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 7

                                    Item {
                                        Layout.fillWidth: true
                                    }

                                    EchoButton {
                                        text: qsTr("Cancel")
                                        ghost: true
                                        onClicked: dialog.choosingUpdate = false
                                    }

                                    EchoButton {
                                        text: qsTr("Add version")
                                        enabled: dialog.selectedUpdateComponents().length > 0
                                        onClicked: {
                                            const componentIds = dialog.selectedUpdateComponents();
                                            if (componentIds.length === 0) {
                                                updateValidation.text = qsTr("Choose at least one processing module.");
                                                return;
                                            }
                                            dialog.updateRequested(dialog.selectedRecipeId, componentIds);
                                            dialog.choosingUpdate = false;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: archiveContent.implicitHeight + 20
                        visible: dialog.selectedRecipe !== null
                        radius: Theme.controlRadius
                        color: Theme.warningSurface

                        RowLayout {
                            id: archiveContent

                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 10
                            spacing: 10

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    text: qsTr("Archive recipe")
                                    color: Theme.textPrimary
                                    font.pixelSize: Theme.fontBody
                                    font.weight: Font.DemiBold
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: qsTr("Remove it from recipe choices while keeping its versions and application history.")
                                    color: Theme.warningText
                                    font.pixelSize: Theme.fontMeta
                                    wrapMode: Text.WordWrap
                                }
                            }

                            EchoButton {
                                text: qsTr("Archive…")
                                ghost: true
                                onClicked: archiveConfirmation.open()
                            }
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: dialog.selectedRecipe === null
                        text: dialog.modelCount() === 0 ? qsTr("No processing recipes to manage") : qsTr("Select a processing recipe")
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }

                    Item {
                        Layout.preferredHeight: 2
                    }
                }
            }
        }
    }

    Dialog {
        id: archiveConfirmation

        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(430, parent.width - 40)
        modal: true
        dim: true
        title: qsTr("Archive processing recipe?")
        standardButtons: Dialog.NoButton

        background: Rectangle {
            radius: Theme.panelRadius
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 12

            Text {
                Layout.fillWidth: true
                text: dialog.selectedRecipeName()
                color: Theme.textPrimary
                font.pixelSize: 15
                font.weight: Font.DemiBold
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("This recipe will leave the picker, but its versions and past applications remain available for history and safe rollback.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Item {
                    Layout.fillWidth: true
                }

                EchoButton {
                    text: qsTr("Cancel")
                    ghost: true
                    onClicked: archiveConfirmation.close()
                }

                EchoButton {
                    text: qsTr("Archive recipe")
                    backgroundColor: Theme.warningText
                    enabled: dialog.selectedRecipe !== null
                    onClicked: {
                        if (!dialog.selectedRecipe)
                            return;
                        dialog.archiveRequested(dialog.selectedRecipeId);
                        archiveConfirmation.close();
                    }
                }
            }
        }
    }
}
