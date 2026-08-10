//! User-owned operations for the selected sound: affinity and explicit album
//! membership. The album popup remains with its trigger and placement owner.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: toolbar

    required property var asset
    required property var userAlbums

    signal affinityRequested(bool liked, int rating)
    signal albumMembershipRequested(var album, bool included)
    signal createAlbumRequested()

    visible: asset !== null
    implicitWidth: operations.implicitWidth + 20
    implicitHeight: 44
    radius: 12
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    Accessible.role: Accessible.ToolBar
    Accessible.name: qsTr("Selected sound actions")

    RowLayout {
        id: operations

        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 10
        anchors.topMargin: 7
        anchors.bottomMargin: 7
        spacing: 3

        Button {
            id: likeButton

            Layout.preferredWidth: 30
            Layout.preferredHeight: 30
            padding: 0
            focusPolicy: Qt.NoFocus
            onClicked: toolbar.affinityRequested(
                toolbar.asset !== null && !toolbar.asset.liked,
                toolbar.asset !== null ? toolbar.asset.rating : 0)

            ToolTip.visible: hovered
            ToolTip.text: toolbar.asset !== null && toolbar.asset.liked
                ? qsTr("Remove Like") : qsTr("Like")
            ToolTip.delay: 500
            Accessible.name: ToolTip.text

            background: Rectangle {
                radius: Theme.compactControlRadius
                color: likeButton.hovered ? Theme.buttonGhostHover : Theme.transparent
            }

            contentItem: Text {
                text: toolbar.asset !== null && toolbar.asset.liked ? "♥" : "♡"
                color: toolbar.asset !== null && toolbar.asset.liked
                    ? "#dc4b6b" : Theme.textSecondary
                font.pixelSize: 17
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        Repeater {
            model: 5

            delegate: Button {
                id: starButton
                required property int index

                readonly property int ratingValue: index + 1
                Layout.preferredWidth: 25
                Layout.preferredHeight: 30
                padding: 0
                focusPolicy: Qt.NoFocus
                onClicked: toolbar.affinityRequested(
                    toolbar.asset !== null && toolbar.asset.liked,
                    toolbar.asset !== null && toolbar.asset.rating === ratingValue
                        ? 0 : ratingValue)

                ToolTip.visible: hovered
                ToolTip.text: qsTr("Set %1 stars").arg(ratingValue)
                ToolTip.delay: 500
                Accessible.name: ToolTip.text

                background: Rectangle {
                    radius: 4
                    color: starButton.hovered
                        ? Theme.buttonGhostHover : Theme.transparent
                }

                contentItem: Text {
                    text: toolbar.asset !== null
                        && starButton.ratingValue <= toolbar.asset.rating ? "★" : "☆"
                    color: toolbar.asset !== null
                        && starButton.ratingValue <= toolbar.asset.rating
                        ? "#d89a16" : Theme.textDisabled
                    font.pixelSize: 15
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        Button {
            id: albumButton

            Layout.preferredWidth: 32
            Layout.preferredHeight: 30
            padding: 0
            focusPolicy: Qt.NoFocus
            onClicked: albumMenu.popup(albumButton, 0, -albumMenu.implicitHeight - 6)

            ToolTip.visible: hovered
            ToolTip.text: qsTr("Add to album")
            ToolTip.delay: 500
            Accessible.name: ToolTip.text

            background: Rectangle {
                radius: Theme.compactControlRadius
                color: albumButton.hovered ? Theme.buttonGhostHover : Theme.transparent
            }

            contentItem: Text {
                text: "▱+"
                color: Theme.textSecondary
                font.pixelSize: 13
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

    }

    Menu {
        id: albumMenu

        Repeater {
            model: toolbar.userAlbums

            delegate: MenuItem {
                required property var modelData

                text: modelData.name
                checkable: true
                checked: toolbar.asset !== null
                    && modelData.memberIds.includes(toolbar.asset.id)
                onTriggered: toolbar.albumMembershipRequested(modelData, checked)
            }
        }

        MenuItem {
            visible: toolbar.userAlbums.length === 0
            enabled: false
            text: qsTr("No albums yet")
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("New album…")
            onTriggered: toolbar.createAlbumRequested()
        }
    }
}
