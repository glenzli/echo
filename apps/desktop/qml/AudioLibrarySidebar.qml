//! Sound Library navigation. System collections and smart albums are a stable
//! browse index; folder management remains in AudioLibraryWorkspace.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: sidebar

    required property var assets
    required property var smartAlbums
    required property var keywordFacets
    required property string selectedFilter

    signal filterRequested(string key)
    signal manageLibraryRequested()

    color: Theme.panel

    function countFor(key: string) : int {
        let count = 0
        const now = Date.now()
        for (let index = 0; index < assets.length; ++index) {
            const asset = assets[index]
            if (key === "all"
                    || (key === "recent"
                        && asset.importedAtMillis >= now - 7 * 86400000)
                    || (key === "liked" && asset.liked)
                    || (key === "five-star" && asset.rating === 5)
                    || (key === "has-text" && asset.textPreview.length > 0)
                    || (key === "missing" && asset.pathStatus === "missing")) {
                count += 1
            }
        }
        return count
    }

    Rectangle {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: 1
        color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 13
        anchors.topMargin: 15
        anchors.bottomMargin: 12
        spacing: 5

        Text {
            Layout.leftMargin: 7
            text: qsTr("LIBRARY")
            color: Theme.textDisabled
            font.pixelSize: Theme.fontMeta
            font.bold: true
            font.letterSpacing: 1.3
        }

        Item { Layout.preferredHeight: 3 }

        Repeater {
            model: [
                { key: "all", label: qsTr("All sounds"), glyph: "▦" },
                { key: "recent", label: qsTr("Recently added"), glyph: "◷" },
                { key: "liked", label: qsTr("Liked"), glyph: "♡" },
                { key: "five-star", label: qsTr("5 stars"), glyph: "☆" }
            ]

            delegate: SoundLibraryRow {
                required property var modelData

                Layout.fillWidth: true
                label: modelData.label
                glyph: modelData.glyph
                count: sidebar.countFor(modelData.key)
                selected: sidebar.selectedFilter === modelData.key
                onActivated: sidebar.filterRequested(modelData.key)
            }
        }

        Item { Layout.preferredHeight: 9 }

        Text {
            Layout.leftMargin: 7
            text: qsTr("SMART COLLECTIONS")
            color: Theme.textDisabled
            font.pixelSize: Theme.fontMeta
            font.bold: true
            font.letterSpacing: 1.3
        }

        Repeater {
            model: [
                { key: "has-text", label: qsTr("With text"), glyph: "“" },
                { key: "missing", label: qsTr("Missing originals"), glyph: "◇" }
            ]

            delegate: SoundLibraryRow {
                required property var modelData

                Layout.fillWidth: true
                label: modelData.label
                glyph: modelData.glyph
                count: sidebar.countFor(modelData.key)
                selected: sidebar.selectedFilter === modelData.key
                onActivated: sidebar.filterRequested(modelData.key)
            }
        }

        Item { Layout.preferredHeight: 9 }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: parent.width
                spacing: 2

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 7
                    Layout.topMargin: 2
                    Layout.bottomMargin: 4
                    text: qsTr("AI KEYWORDS")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 1.3
                }

                Repeater {
                    model: sidebar.keywordFacets

                    delegate: SoundLibraryRow {
                        required property var modelData

                        Layout.fillWidth: true
                        label: modelData.label
                        glyph: "✦"
                        count: modelData.count
                        selected: sidebar.selectedFilter === "keyword:" + modelData.key
                        onActivated: sidebar.filterRequested("keyword:" + modelData.key)
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 7
                    Layout.topMargin: 10
                    Layout.bottomMargin: 4
                    text: qsTr("SOUND ALBUMS")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 1.3
                }

                Repeater {
                    model: sidebar.smartAlbums

                    delegate: SoundLibraryRow {
                        required property var modelData

                        Layout.fillWidth: true
                        label: modelData.label
                        glyph: modelData.aiSuggested ? "✦" : "▱"
                        count: modelData.count
                        selected: sidebar.selectedFilter === modelData.key
                        onActivated: sidebar.filterRequested(modelData.key)
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 8
                    Layout.rightMargin: 8
                    visible: sidebar.smartAlbums.length === 0
                    text: qsTr("AI-discovered places, people, and events will appear here as evidence becomes available.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                    lineHeight: 1.25
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Button {
            Layout.fillWidth: true
            implicitHeight: 34
            onClicked: sidebar.manageLibraryRequested()

            background: Rectangle {
                radius: Theme.controlRadius
                color: parent.hovered ? Theme.surfaceSubtle : Theme.transparent
            }

            contentItem: RowLayout {
                spacing: 8

                EchoIcon {
                    source: "qrc:/EchoDesktop/icons/folder.svg"
                    size: 15
                    color: Theme.textSecondary
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Manage folders")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                }
            }
        }
    }
}
