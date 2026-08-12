//! Controlled picker for reusable processing recipes. The caller owns the
//! authoritative recipe model and selected identity; this component only
//! derives a searchable presentation and emits selection intent.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: picker

    // Entries accept { id, name, revisionId, revisionNumber, components,
    // updatedAtMillis }. Replace the array or bump modelRevision after an
    // in-place mutation.
    property var recipeModel: []
    property int modelRevision: 0
    property string selectedRecipeId: ""

    signal recipeSelected(string recipeId)

    implicitWidth: 380
    implicitHeight: 330
    radius: Theme.controlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.border
    clip: true

    function modelCount(): int {
        if (!picker.recipeModel)
            return 0;
        if (picker.recipeModel.count !== undefined)
            return picker.recipeModel.count;
        return picker.recipeModel.length !== undefined ? picker.recipeModel.length : 0;
    }

    function modelEntry(index: int): var {
        if (picker.recipeModel.get !== undefined)
            return picker.recipeModel.get(index);
        return picker.recipeModel[index];
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
        if (componentId === "channelRepair")
            return qsTr("Channel repair");
        if (componentId === "equalizer")
            return qsTr("Equalizer");
        if (componentId === "dynamics")
            return qsTr("Dynamics");
        if (componentId === "space")
            return qsTr("Space");
        if (componentId === "sceneVfx")
            return qsTr("Scene VFX");
        if (componentId === "delayVfx")
            return qsTr("Delay VFX");
        if (componentId === "modulationVfx")
            return qsTr("Modulation VFX");
        if (componentId === "transformVfx")
            return qsTr("Transform VFX");
        if (componentId === "digitalDegradeVfx")
            return qsTr("Digital Degrade");
        if (componentId === "master")
            return qsTr("Master output");
        return componentId;
    }

    function componentSummary(components: var): string {
        if (!components || components.length === undefined)
            return "";
        const titles = [];
        for (let index = 0; index < components.length; ++index) {
            const component = components[index];
            const key = typeof component === "string" ? component : String(component.id || component.componentId || "");
            const title = typeof component === "object" && component.title ? String(component.title) : picker.componentTitle(key);
            if (title.length > 0)
                titles.push(title);
        }
        return titles.join(" · ");
    }

    function rebuildProjection(): void {
        const query = searchField.text.trim().toLocaleLowerCase();
        recipeProjection.clear();
        for (let index = 0; index < picker.modelCount(); ++index) {
            const entry = picker.modelEntry(index);
            if (!entry)
                continue;
            const recipeId = String(entry.id || entry.recipeId || "");
            const title = String(entry.name || qsTr("Untitled processing recipe"));
            const components = picker.componentSummary(entry.components);
            const searchable = (title + " " + components).toLocaleLowerCase();
            if (query.length > 0 && searchable.indexOf(query) === -1)
                continue;
            const revisionNumber = Number(entry.revisionNumber || 1);
            recipeProjection.append({
                recipeId: recipeId,
                title: title,
                versionText: qsTr("Version %1").arg(revisionNumber),
                componentSummary: components,
                updatedAtMillis: Number(entry.updatedAtMillis || 0)
            });
        }
    }

    readonly property int observedModelCount: modelCount()

    onRecipeModelChanged: rebuildProjection()
    onModelRevisionChanged: rebuildProjection()
    onObservedModelCountChanged: rebuildProjection()
    Component.onCompleted: rebuildProjection()

    ListModel {
        id: recipeProjection
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: 10
            spacing: 7

            Text {
                Layout.fillWidth: true
                text: qsTr("Processing recipes")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontSection
                font.weight: Font.DemiBold
            }

            EchoTextField {
                id: searchField

                Layout.fillWidth: true
                implicitHeight: Theme.compactControlHeight
                placeholderText: qsTr("Search processing recipes")
                Accessible.name: placeholderText
                onTextChanged: picker.rebuildProjection()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ListView {
            id: recipeList

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 7
            model: recipeProjection
            spacing: 4
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true
            activeFocusOnTab: true
            keyNavigationEnabled: true

            ScrollBar.vertical: ScrollBar {
                policy: recipeList.contentHeight > recipeList.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: ItemDelegate {
                id: recipeRow

                required property string recipeId
                required property string title
                required property string versionText
                required property string componentSummary

                width: recipeList.width - 5
                height: 68
                padding: 0
                hoverEnabled: true
                Accessible.name: recipeRow.title + ", " + recipeRow.versionText
                onClicked: picker.recipeSelected(recipeRow.recipeId)

                background: Rectangle {
                    radius: Theme.compactControlRadius
                    color: recipeRow.recipeId === picker.selectedRecipeId ? Theme.accentSurfaceQuiet : recipeRow.hovered ? Theme.surfaceSubtle : Theme.transparent
                    border.width: recipeRow.recipeId === picker.selectedRecipeId ? 1 : 0
                    border.color: Theme.accent
                }

                contentItem: RowLayout {
                    spacing: 9

                    Rectangle {
                        Layout.preferredWidth: 32
                        Layout.preferredHeight: 32
                        radius: 8
                        color: Theme.surfaceSelected

                        EchoIcon {
                            anchors.centerIn: parent
                            source: "qrc:/EchoDesktop/icons/equalizer.svg"
                            size: 17
                            color: recipeRow.recipeId === picker.selectedRecipeId ? Theme.accentSelectionText : Theme.textSecondary
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                Layout.fillWidth: true
                                text: recipeRow.title
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Text {
                                text: recipeRow.versionText
                                color: Theme.textDisabled
                                font.pixelSize: Theme.fontMeta
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: recipeRow.componentSummary.length > 0 ? recipeRow.componentSummary : qsTr("No processing modules")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: recipeList.count === 0
                width: Math.min(260, recipeList.width - 24)
                text: picker.modelCount() === 0 ? qsTr("No processing recipes yet") : qsTr("No matching processing recipes")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }
    }
}
