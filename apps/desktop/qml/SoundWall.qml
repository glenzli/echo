//! Parallel sound browsing surface. Window-level search, sort and density
//! controls live in MainTitleBar; this owner renders the cards and empty state.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: wall

    required property var assets
    required property var selectedAsset
    required property string searchText
    required property int preferredCardWidth

    signal assetSelected(var asset)
    signal assetOpened(var asset)
    signal affinityRequested(var asset, bool liked, int rating)

    color: Theme.window

    GridView {
        id: soundGrid

        readonly property int spacing: 12
        readonly property int columns: Math.max(1,
            Math.floor((width + spacing) / (wall.preferredCardWidth + spacing)))

        anchors.fill: parent
        anchors.margins: 14
        clip: true
        model: wall.assets
        cellWidth: width / columns
        cellHeight: 218
        boundsBehavior: Flickable.StopAtBounds

        delegate: Item {
            required property var modelData

            width: soundGrid.cellWidth
            height: soundGrid.cellHeight

            SoundCard {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.rightMargin: soundGrid.spacing
                height: 206
                entry: modelData
                selected: wall.selectedAsset !== null
                    && wall.selectedAsset.id === modelData.id
                onActivated: wall.assetSelected(modelData)
                onOpened: wall.assetOpened(modelData)
                onAffinityRequested: function(liked, rating) {
                    wall.affinityRequested(modelData, liked, rating)
                }
            }
        }

        ScrollBar.vertical: ScrollBar {}
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 60, 360)
        spacing: 10
        visible: wall.assets.count === 0

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
}
