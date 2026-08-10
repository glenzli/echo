//! Searchable effect catalog. The caller owns the authoritative catalog and
//! decides whether an add request is accepted.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: catalog

    // Each entry has effectId, title, summary, categoryId, categoryTitle,
    // iconSource, and an optional available flag. Replace the array or bump
    // modelRevision after mutating an object in place.
    property var effectModel: []
    property int modelRevision: 0
    property string selectedCategoryId: "all"
    property alias searchText: searchField.text

    signal effectAddRequested(string effectId)

    implicitWidth: 340
    implicitHeight: 420
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong
    clip: true

    function modelCount(): int {
        if (!catalog.effectModel)
            return 0
        if (catalog.effectModel.count !== undefined)
            return catalog.effectModel.count
        return catalog.effectModel.length !== undefined
            ? catalog.effectModel.length : 0
    }

    function modelEntry(index: int): var {
        if (catalog.effectModel.get !== undefined)
            return catalog.effectModel.get(index)
        return catalog.effectModel[index]
    }

    function rebuildProjection(): void {
        const query = searchField.text.trim().toLocaleLowerCase()
        const categories = ({})
        let selectedCategoryExists = catalog.selectedCategoryId === "all"

        categoryProjection.clear()
        categoryProjection.append({
            categoryId: "all",
            categoryTitle: qsTr("All")
        })
        effectProjection.clear()

        for (let index = 0; index < catalog.modelCount(); ++index) {
            const entry = catalog.modelEntry(index)
            if (!entry)
                continue

            const categoryId = String(entry.categoryId || "other")
            const categoryTitle = String(entry.categoryTitle || qsTr("Other"))
            if (!categories[categoryId]) {
                categories[categoryId] = true
                categoryProjection.append({
                    categoryId: categoryId,
                    categoryTitle: categoryTitle
                })
            }
            if (categoryId === catalog.selectedCategoryId)
                selectedCategoryExists = true

            const title = String(entry.title || "")
            const summary = String(entry.summary || "")
            const matchesCategory = catalog.selectedCategoryId === "all"
                || categoryId === catalog.selectedCategoryId
            const searchableText = (title + " " + summary + " "
                + categoryTitle).toLocaleLowerCase()
            if (matchesCategory && (query.length === 0
                    || searchableText.indexOf(query) !== -1)) {
                effectProjection.append({
                    effectId: String(entry.effectId || ""),
                    title: title,
                    summary: summary,
                    categoryTitle: categoryTitle,
                    iconSource: String(entry.iconSource || ""),
                    available: entry.available === undefined
                        ? true : Boolean(entry.available)
                })
            }
        }

        if (!selectedCategoryExists)
            catalog.selectedCategoryId = "all"
    }

    readonly property int observedModelCount: modelCount()

    onEffectModelChanged: rebuildProjection()
    onModelRevisionChanged: rebuildProjection()
    onObservedModelCountChanged: rebuildProjection()
    onSelectedCategoryIdChanged: rebuildProjection()
    Component.onCompleted: rebuildProjection()

    ListModel { id: categoryProjection }
    ListModel { id: effectProjection }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        AdjustmentPanelHeader {
            Layout.fillWidth: true
            title: qsTr("Add effect")
            iconSource: "qrc:/EchoDesktop/icons/equalizer.svg"
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            Layout.topMargin: 8
            Layout.bottomMargin: 7
            spacing: 7

            EchoTextField {
                id: searchField

                Layout.fillWidth: true
                implicitHeight: Theme.compactControlHeight
                placeholderText: qsTr("Search effects")
                onTextChanged: catalog.rebuildProjection()
            }

            ListView {
                id: categoryList

                Layout.fillWidth: true
                Layout.preferredHeight: 26
                orientation: ListView.Horizontal
                model: categoryProjection
                spacing: 5
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                delegate: Rectangle {
                    id: categoryChip

                    required property string categoryId
                    required property string categoryTitle

                    width: categoryLabel.implicitWidth + 18
                    height: 24
                    radius: Theme.compactControlRadius
                    color: categoryChip.categoryId === catalog.selectedCategoryId
                        ? Theme.accentSurfaceQuiet
                        : categoryHover.hovered ? Theme.buttonGhostHover
                                                : Theme.transparent
                    border.width: categoryChip.categoryId
                        === catalog.selectedCategoryId ? 1 : 0
                    border.color: Theme.accent

                    Text {
                        id: categoryLabel

                        anchors.centerIn: parent
                        text: categoryChip.categoryTitle
                        color: categoryChip.categoryId
                            === catalog.selectedCategoryId
                            ? Theme.accentSelectionText : Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }

                    HoverHandler { id: categoryHover }
                    TapHandler {
                        onTapped: catalog.selectedCategoryId
                            = categoryChip.categoryId
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ListView {
            id: effectList

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 7
            model: effectProjection
            spacing: 4
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true

            ScrollBar.vertical: ScrollBar {
                policy: effectList.contentHeight > effectList.height
                    ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: Rectangle {
                id: effectRow

                required property string effectId
                required property string title
                required property string summary
                required property string categoryTitle
                required property string iconSource
                required property bool available

                width: effectList.width - 5
                height: 58
                radius: Theme.compactControlRadius
                color: effectHover.hovered ? Theme.surfaceSubtle
                                           : Theme.transparent

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 9
                    anchors.rightMargin: 7
                    spacing: 8

                    EchoIcon {
                        source: effectRow.iconSource
                        size: 17
                        color: effectRow.available ? Theme.textSecondary
                                                   : Theme.textDisabled
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            Layout.fillWidth: true
                            text: effectRow.title
                            color: effectRow.available ? Theme.textPrimary
                                                       : Theme.textDisabled
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: effectRow.summary.length > 0
                                ? effectRow.summary : effectRow.categoryTitle
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }

                    EchoButton {
                        implicitWidth: effectRow.available ? 56 : 82
                        implicitHeight: 26
                        text: effectRow.available ? qsTr("Add")
                                                  : qsTr("Unavailable")
                        ghost: true
                        enabled: effectRow.available
                        onClicked: catalog.effectAddRequested(effectRow.effectId)
                    }
                }

                HoverHandler { id: effectHover }
            }

            Text {
                anchors.centerIn: parent
                visible: effectList.count === 0
                text: qsTr("No effects found")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontBody
            }
        }
    }
}
