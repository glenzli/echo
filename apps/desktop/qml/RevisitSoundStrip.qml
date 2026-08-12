//! One bounded horizontal memory row. SoundCard remains the canonical visual
//! surrogate; this owner owns section geometry and horizontal navigation.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

ColumnLayout {
    id: strip

    required property string title
    required property string subtitle
    required property var assets
    required property string selectedAssetId
    property int cardWidth: 224
    property int cardHeight: 190
    property string density: "overview"

    signal assetSelected(var asset, int modifiers)
    signal assetOpened(var asset)

    visible: assets.length > 0
    spacing: 9

    RowLayout {
        Layout.fillWidth: true
        spacing: 9

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Text {
                Layout.fillWidth: true
                text: strip.title
                color: Theme.textPrimary
                font.pixelSize: 15
                font.bold: true
            }

            Text {
                Layout.fillWidth: true
                visible: strip.subtitle.length > 0
                text: strip.subtitle
                color: Theme.textMuted
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        Text {
            text: String(strip.assets.length)
            color: Theme.textDisabled
            font.pixelSize: Theme.fontMeta
        }
    }

    ListView {
        id: row

        Layout.fillWidth: true
        Layout.preferredHeight: strip.cardHeight + 4
        orientation: ListView.Horizontal
        spacing: 12
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: strip.assets
        reuseItems: true

        delegate: SoundCard {
            required property var modelData

            width: strip.cardWidth
            height: strip.cardHeight
            entry: modelData
            selected: modelData.id === strip.selectedAssetId
            density: strip.density
            displayHeight: strip.cardHeight
            onActivated: modifiers => strip.assetSelected(modelData, modifiers)
            onOpened: strip.assetOpened(modelData)
        }

        ScrollBar.horizontal: ScrollBar {
            policy: ScrollBar.AsNeeded
        }
    }
}
