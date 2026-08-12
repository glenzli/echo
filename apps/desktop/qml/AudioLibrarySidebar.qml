//! Sound Library navigation. User albums are durable facts, while suggested
//! albums remain a separate, rebuildable browse index.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: sidebar

    required property var assets
    required property var userAlbums
    required property var suggestedAlbums
    required property string selectedFilter
    property string albumError: ""

    signal filterRequested(string key)
    signal manageLibraryRequested
    signal createAlbumRequested(string name, var memberIds)
    signal renameAlbumRequested(var albumId, string name)
    signal deleteAlbumRequested(var albumId)
    signal saveSuggestedAlbumRequested(var album, string name)

    property string albumEditorMode: ""
    property var albumEditorTarget: null
    property var pendingDeleteAlbum: null
    property var pendingInitialMemberIds: []

    color: Theme.panel

    function albumReason(album: var): string {
        if (album.evidence === "original") {
            return album.facet === "time" ? qsTr("Shared recording day") : qsTr("Embedded location");
        }
        if (album.facet === "place") {
            return qsTr("Shared AI place");
        }
        if (album.facet === "event") {
            return qsTr("Shared AI event");
        }
        if (album.facet === "person") {
            return qsTr("Shared AI people hint");
        }
        return qsTr("Related by AI");
    }

    function countFor(key: string): int {
        let count = 0;
        const now = Date.now();
        for (let index = 0; index < assets.length; ++index) {
            const asset = assets[index];
            if (key === "all" || (key === "recent" && asset.importedAtMillis >= now - 7 * 86400000) || (key === "listened" && Number(asset.lastListenedAtMillis || 0) > 0) || (key === "liked" && asset.liked) || (key === "five-star" && asset.rating === 5) || (key === "has-speech" && asset.textPreview.length > 0) || (key === "missing" && asset.pathStatus === "missing")) {
                count += 1;
            }
        }
        return count;
    }

    function editAlbum(mode: string, target: var, title: string, action: string, name: string): void {
        albumEditorMode = mode;
        albumEditorTarget = target;
        albumNameDialog.show(title, action, name);
    }

    function beginCreate(memberIds: var): void {
        pendingInitialMemberIds = memberIds || [];
        editAlbum("create", null, qsTr("New album"), qsTr("Create"), "");
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

        Item {
            Layout.preferredHeight: 3
        }

        Repeater {
            model: [
                {
                    key: "all",
                    label: qsTr("All sounds"),
                    iconSource: "qrc:/EchoDesktop/icons/review-grid.svg"
                },
                {
                    key: "recent",
                    label: qsTr("Recently added"),
                    iconSource: "qrc:/EchoDesktop/icons/clock.svg"
                },
                {
                    key: "listened",
                    label: qsTr("Recently listened"),
                    iconSource: "qrc:/EchoDesktop/icons/history.svg"
                },
                {
                    key: "liked",
                    label: qsTr("Liked"),
                    iconSource: "qrc:/EchoDesktop/icons/heart.svg"
                },
                {
                    key: "five-star",
                    label: qsTr("5 stars"),
                    iconSource: "qrc:/EchoDesktop/icons/star.svg"
                }
            ]

            delegate: SoundLibraryRow {
                required property var modelData

                Layout.fillWidth: true
                label: modelData.label
                iconSource: modelData.iconSource
                count: sidebar.countFor(modelData.key)
                selected: sidebar.selectedFilter === modelData.key
                onActivated: sidebar.filterRequested(modelData.key)
            }
        }

        Item {
            Layout.preferredHeight: 9
        }

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
                {
                    key: "has-speech",
                    label: qsTr("With speech"),
                    iconSource: "qrc:/EchoDesktop/icons/mic.svg"
                },
                {
                    key: "missing",
                    label: qsTr("Missing originals"),
                    iconSource: "qrc:/EchoDesktop/icons/source-missing.svg"
                }
            ]

            delegate: SoundLibraryRow {
                required property var modelData

                Layout.fillWidth: true
                label: modelData.label
                iconSource: modelData.iconSource || ""
                count: sidebar.countFor(modelData.key)
                selected: sidebar.selectedFilter === modelData.key
                onActivated: sidebar.filterRequested(modelData.key)
            }
        }

        Item {
            Layout.preferredHeight: 9
        }

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
                    text: qsTr("ALBUMS")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 1.3
                }

                Repeater {
                    model: sidebar.userAlbums

                    delegate: SoundAlbumRow {
                        required property var modelData

                        Layout.fillWidth: true
                        label: modelData.name
                        count: modelData.count
                        selected: sidebar.selectedFilter === "user-album:" + modelData.id
                        onActivated: sidebar.filterRequested("user-album:" + modelData.id)
                        onRenameRequested: sidebar.editAlbum("rename", modelData, qsTr("Rename album"), qsTr("Rename"), modelData.name)
                        onDeleteRequested: {
                            sidebar.pendingDeleteAlbum = modelData;
                            deleteDialog.open();
                        }
                    }
                }

                Button {
                    id: newAlbumButton

                    Layout.fillWidth: true
                    implicitHeight: 32
                    padding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: sidebar.beginCreate([])

                    background: Rectangle {
                        radius: Theme.controlRadius
                        color: newAlbumButton.hovered ? Theme.buttonGhostHover : Theme.transparent
                    }

                    contentItem: RowLayout {
                        spacing: 8

                        EchoIcon {
                            Layout.preferredWidth: 17
                            source: "qrc:/EchoDesktop/icons/plus.svg"
                            size: 15
                            color: Theme.accent
                        }

                        Text {
                            Layout.fillWidth: true
                            text: qsTr("New album")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 7
                    Layout.rightMargin: 7
                    Layout.topMargin: 6
                    Layout.bottomMargin: 6
                    Layout.preferredHeight: 1
                    color: Theme.border
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 7
                    Layout.bottomMargin: 4
                    text: qsTr("SUGGESTED")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.bold: true
                    font.letterSpacing: 1.3
                }

                Repeater {
                    model: sidebar.suggestedAlbums

                    delegate: SoundAlbumRow {
                        required property var modelData

                        Layout.fillWidth: true
                        label: modelData.label
                        subtitle: sidebar.albumReason(modelData)
                        suggested: true
                        count: modelData.count
                        selected: sidebar.selectedFilter === "suggested-album:" + modelData.key
                        onActivated: sidebar.filterRequested("suggested-album:" + modelData.key)
                        onSaveRequested: sidebar.editAlbum("save-suggestion", modelData, qsTr("Save suggested album"), qsTr("Save"), modelData.label)
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 8
                    Layout.rightMargin: 8
                    visible: sidebar.suggestedAlbums.length === 0
                    text: qsTr("Album suggestions will appear when at least two sounds share time, place, event, or people.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                    lineHeight: 1.25
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 8
                    Layout.rightMargin: 8
                    Layout.topMargin: 6
                    visible: sidebar.albumError.length > 0
                    text: sidebar.albumError
                    color: Theme.warningText
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Button {
            id: manageFoldersButton

            Layout.fillWidth: true
            implicitHeight: 34
            onClicked: sidebar.manageLibraryRequested()

            background: Rectangle {
                radius: Theme.controlRadius
                color: manageFoldersButton.hovered ? Theme.surfaceSubtle : Theme.transparent
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

    SoundAlbumNameDialog {
        id: albumNameDialog

        onSubmitted: function (name) {
            if (sidebar.albumEditorMode === "create") {
                sidebar.createAlbumRequested(name, sidebar.pendingInitialMemberIds);
            } else if (sidebar.albumEditorMode === "rename" && sidebar.albumEditorTarget !== null) {
                sidebar.renameAlbumRequested(sidebar.albumEditorTarget.id, name);
            } else if (sidebar.albumEditorMode === "save-suggestion" && sidebar.albumEditorTarget !== null) {
                sidebar.saveSuggestedAlbumRequested(sidebar.albumEditorTarget, name);
            }
        }
    }

    Dialog {
        id: deleteDialog

        parent: Overlay.overlay
        modal: true
        dim: true
        width: 380
        height: 190
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        padding: 0

        background: Rectangle {
            radius: 12
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            anchors.fill: parent
            anchors.margins: 20
            spacing: 12

            Text {
                Layout.fillWidth: true
                text: qsTr("Delete album?")
                color: Theme.textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("The sounds and their original files will not be deleted.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            Item {
                Layout.fillHeight: true
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
                    onClicked: deleteDialog.close()
                }

                EchoButton {
                    text: qsTr("Delete")
                    backgroundColor: "#b64d52"
                    onClicked: {
                        if (sidebar.pendingDeleteAlbum !== null) {
                            sidebar.deleteAlbumRequested(sidebar.pendingDeleteAlbum.id);
                        }
                        deleteDialog.close();
                    }
                }
            }
        }
    }
}
