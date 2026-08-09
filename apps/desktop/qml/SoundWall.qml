//! Parallel sound browsing surface. It owns search, sort and card geometry;
//! collection admission and authoritative asset projection stay with Audio Space.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: wall

    required property var assets
    required property var selectedAsset
    required property string collectionLabel
    property int preferredCardWidth: 286

    signal assetSelected(var asset)
    signal assetOpened(var asset)
    signal affinityRequested(var asset, bool liked, int rating)
    signal searchRequested(string text)
    signal sortRequested(string mode)

    color: Theme.window

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 54
            color: Theme.chrome

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: Theme.border
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 12
                spacing: 9

                ColumnLayout {
                    spacing: 0

                    Text {
                        text: wall.collectionLabel
                        color: Theme.textPrimary
                        font.pixelSize: 13
                        font.bold: true
                    }

                    Text {
                        text: qsTr("%1 sounds").arg(wall.assets.count)
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontMeta
                    }
                }

                EchoTextField {
                    id: searchField
                    Layout.fillWidth: true
                    Layout.maximumWidth: 270
                    Layout.leftMargin: 8
                    placeholderText: qsTr("Search text, events, or filenames…")
                    onTextChanged: wall.searchRequested(text)
                }

                Item { Layout.fillWidth: true }

                ComboBox {
                    id: sortBox
                    Layout.preferredWidth: 104
                    implicitHeight: Theme.compactControlHeight
                    model: [qsTr("Date"), qsTr("Duration"), qsTr("Rating")]
                    onCurrentIndexChanged: wall.sortRequested(
                        ["date", "duration", "rating"][currentIndex])

                    contentItem: Text {
                        leftPadding: 10
                        rightPadding: 24
                        text: sortBox.displayText
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontMeta
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    indicator: Text {
                        x: sortBox.width - width - 9
                        y: Math.round((sortBox.height - height) / 2) - 1
                        text: "⌄"
                        color: Theme.textSecondary
                        font.pixelSize: 13
                    }

                    background: Rectangle {
                        radius: Theme.controlRadius
                        color: sortBox.down ? Theme.controlPressed : Theme.control
                        border.color: sortBox.activeFocus
                            ? Theme.accent : Theme.buttonBorder
                    }
                }

                Text {
                    text: qsTr("Size")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                Slider {
                    Layout.preferredWidth: 76
                    from: 230
                    to: 390
                    stepSize: 10
                    value: wall.preferredCardWidth
                    onMoved: wall.preferredCardWidth = value
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

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
                    text: searchField.text.trim().length > 0
                        ? qsTr("No matching sounds") : qsTr("This collection is empty")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.bold: true
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    Layout.fillWidth: true
                    text: searchField.text.trim().length > 0
                        ? qsTr("Try another word, event, or filename.")
                        : qsTr("Imported recordings will appear here as sound cards.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontBody
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }
}
