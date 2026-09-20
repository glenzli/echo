//! Audio Space home: bounded listening continuity, real user albums and
//! source-date memories. It composes read-only projections and delegates sound
//! rendering to SoundCard.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: dashboard

    required property var revisitState
    required property var selectedAsset

    signal assetSelected(var asset, int modifiers)
    signal assetOpened(var asset)
    signal albumOpened(var album)

    color: Theme.window

    readonly property bool empty: revisitState.visibleAssetCount === 0 && revisitState.albumCards.length === 0

    Flickable {
        id: scroller

        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight + 46
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: content

            width: scroller.width
            spacing: 24

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 8
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                spacing: 14

                Rectangle {
                    Layout.preferredWidth: 42
                    Layout.preferredHeight: 42
                    radius: 13
                    color: Theme.surfaceSelected

                    EchoIcon {
                        anchors.centerIn: parent
                        source: "qrc:/EchoDesktop/icons/history.svg"
                        size: 21
                        color: Theme.accent
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Listen again")
                        color: Theme.textPrimary
                        font.pixelSize: 21
                        font.bold: true
                    }

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Continue listening and return to moments already in your library.")
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontBody
                    }
                }
            }

            RevisitSoundStrip {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 20
                title: qsTr("Continue listening")
                subtitle: qsTr("Resume from the source-time position where you stopped.")
                assets: dashboard.revisitState.continueListening
                selectedAssetId: dashboard.selectedAsset !== null ? dashboard.selectedAsset.id : ""
                cardWidth: 278
                cardHeight: 218
                density: "browse"
                onAssetSelected: function (asset, modifiers) {
                    dashboard.assetSelected(asset, modifiers);
                }
                onAssetOpened: asset => dashboard.assetOpened(asset)
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 20
                visible: dashboard.revisitState.albumCards.length > 0
                spacing: 9

                RowLayout {
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Your collections")
                            color: Theme.textPrimary
                            font.pixelSize: 15
                            font.bold: true
                        }

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("Collections you chose to keep stay stable as analysis changes.")
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontMeta
                        }
                    }

                    Text {
                        text: String(dashboard.revisitState.albumCards.length)
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontMeta
                    }
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 150
                    orientation: ListView.Horizontal
                    spacing: 12
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    model: dashboard.revisitState.albumCards
                    reuseItems: true

                    delegate: RevisitAlbumCard {
                        required property var modelData

                        width: 214
                        height: 146
                        album: modelData
                        onActivated: dashboard.albumOpened(modelData)
                    }

                    ScrollBar.horizontal: ScrollBar {
                        policy: ScrollBar.AsNeeded
                    }
                }
            }

            RevisitSoundStrip {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 20
                title: qsTr("On this day")
                subtitle: qsTr("Recordings from this date in earlier years.")
                assets: dashboard.revisitState.onThisDay
                selectedAssetId: dashboard.selectedAsset !== null ? dashboard.selectedAsset.id : ""
                cardWidth: 224
                cardHeight: 190
                density: "overview"
                onAssetSelected: function (asset, modifiers) {
                    dashboard.assetSelected(asset, modifiers);
                }
                onAssetOpened: asset => dashboard.assetOpened(asset)
            }

            RevisitSoundStrip {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 20
                title: qsTr("Recently listened")
                subtitle: qsTr("Completed sounds and moments without a pending resume point.")
                assets: dashboard.revisitState.recentlyListened
                selectedAssetId: dashboard.selectedAsset !== null ? dashboard.selectedAsset.id : ""
                cardWidth: 224
                cardHeight: 190
                density: "overview"
                onAssetSelected: function (asset, modifiers) {
                    dashboard.assetSelected(asset, modifiers);
                }
                onAssetOpened: asset => dashboard.assetOpened(asset)
            }

            RevisitSoundStrip {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 20
                title: qsTr("New in your library")
                subtitle: qsTr("A bounded view of the newest available originals.")
                assets: dashboard.revisitState.recentlyAdded
                selectedAssetId: dashboard.selectedAsset !== null ? dashboard.selectedAsset.id : ""
                cardWidth: 224
                cardHeight: 190
                density: "overview"
                onAssetSelected: function (asset, modifiers) {
                    dashboard.assetSelected(asset, modifiers);
                }
                onAssetOpened: asset => dashboard.assetOpened(asset)
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                Layout.topMargin: 72
                visible: dashboard.empty
                spacing: 10

                Rectangle {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: 58
                    Layout.preferredHeight: 58
                    radius: 18
                    color: Theme.surfaceSubtle

                    EchoIcon {
                        anchors.centerIn: parent
                        source: "qrc:/EchoDesktop/icons/history.svg"
                        size: 25
                        color: Theme.textDisabled
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Your listening memories will gather here")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.bold: true
                    horizontalAlignment: Text.AlignHCenter
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Listen to a sound or create a collection. Echo will keep the original source and your place in it.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontBody
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 12
            }
        }

        ScrollBar.vertical: ScrollBar {}
    }
}
