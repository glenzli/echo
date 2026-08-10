//! Parallel sound browsing surface. The bottom toolbar owns global browse
//! state; this owner renders density-aware cards and selected-sound actions.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: wall

    required property var assets
    required property var selectedAsset
    required property var selectedAssetIds
    required property string searchText
    required property int preferredCardWidth
    required property string density
    required property var userAlbums

    readonly property int cardHeight: density === "overview"
        ? 184 : density === "rich" ? 292 : 228

    signal assetSelectionRequested(var asset, int modifiers)
    signal assetOpened(var asset)
    signal processingRecipeRequested()
    signal selectionClearRequested()
    signal affinityRequested(var asset, bool liked, int rating)
    signal albumMembershipRequested(var asset, var album, bool included)
    signal createAlbumRequested()

    color: Theme.window

    GridView {
        id: soundGrid

        readonly property int spacing: 12
        readonly property int columns: Math.max(1,
            Math.round((width + spacing) / (wall.preferredCardWidth + spacing)))

        anchors.fill: parent
        anchors.margins: 14
        clip: true
        model: wall.assets
        cellWidth: width / columns
        cellHeight: wall.cardHeight + spacing
        boundsBehavior: Flickable.StopAtBounds
        bottomMargin: singleSelectionToolbar.visible
            ? singleSelectionToolbar.height + 42
            : multiSelectionToolbar.visible ? multiSelectionToolbar.height + 42 : 18

        delegate: Item {
            required property var modelData

            width: soundGrid.cellWidth
            height: soundGrid.cellHeight

            SoundCard {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.rightMargin: soundGrid.spacing
                height: wall.cardHeight
                entry: modelData
                selected: wall.selectedAssetIds.indexOf(modelData.id) >= 0
                density: wall.density
                displayHeight: wall.cardHeight
                onActivated: modifiers => wall.assetSelectionRequested(modelData, modifiers)
                onOpened: wall.assetOpened(modelData)
            }
        }

        ScrollBar.vertical: ScrollBar {}
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 60, 360)
        spacing: 10
        visible: wall.assets.length === 0

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 58
            Layout.preferredHeight: 58
            radius: 18
            color: Theme.surfaceSubtle

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/waveform.svg"
                size: 26
                color: Theme.textDisabled
            }
        }

        Text {
            Layout.fillWidth: true
            text: wall.searchText.trim().length > 0
                ? qsTr("No matching sounds") : qsTr("This collection is empty")
            color: Theme.textPrimary
            font.pixelSize: 16
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: wall.searchText.trim().length > 0
                ? qsTr("Try another word, event, or filename.")
                : qsTr("Imported recordings will appear here as sound cards.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    SoundSelectionToolbar {
        id: singleSelectionToolbar

        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.rightMargin: 20
        anchors.bottomMargin: 18
        asset: wall.selectedAsset
        visible: wall.selectedAsset !== null && wall.selectedAssetIds.length <= 1
        userAlbums: wall.userAlbums
        z: 20
        onAffinityRequested: function(liked, rating) {
            wall.affinityRequested(wall.selectedAsset, liked, rating)
        }
        onAlbumMembershipRequested: function(album, included) {
            wall.albumMembershipRequested(wall.selectedAsset, album, included)
        }
        onCreateAlbumRequested: wall.createAlbumRequested()
    }

    SoundMultiSelectionToolbar {
        id: multiSelectionToolbar

        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.rightMargin: 20
        anchors.bottomMargin: 18
        selectedCount: wall.selectedAssetIds.length
        visible: selectedCount > 1
        z: 20
        onApplyRecipeRequested: wall.processingRecipeRequested()
        onClearRequested: wall.selectionClearRequested()
    }
}
