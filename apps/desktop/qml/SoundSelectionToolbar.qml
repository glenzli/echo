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
    signal createAlbumRequested
    signal assembleRequested

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
            onClicked: toolbar.affinityRequested(toolbar.asset !== null && !toolbar.asset.liked, toolbar.asset !== null ? toolbar.asset.rating : 0)

            ToolTip.visible: hovered
            ToolTip.text: toolbar.asset !== null && toolbar.asset.liked ? qsTr("Remove Like") : qsTr("Like")
            ToolTip.delay: 500
            Accessible.name: ToolTip.text

            background: Rectangle {
                radius: Theme.compactControlRadius
                color: toolbar.asset !== null && toolbar.asset.liked ? Theme.likeSurface : likeButton.hovered ? Theme.buttonGhostHover : Theme.transparent
            }

            contentItem: EchoIcon {
                source: toolbar.asset !== null && toolbar.asset.liked ? "qrc:/EchoDesktop/icons/heart-filled.svg" : "qrc:/EchoDesktop/icons/heart.svg"
                color: toolbar.asset !== null && toolbar.asset.liked ? Theme.likeAccent : Theme.textSecondary
                size: 16
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
                onClicked: toolbar.affinityRequested(toolbar.asset !== null && toolbar.asset.liked, toolbar.asset !== null && toolbar.asset.rating === ratingValue ? 0 : ratingValue)

                ToolTip.visible: hovered
                ToolTip.text: qsTr("Set %1 stars").arg(ratingValue)
                ToolTip.delay: 500
                Accessible.name: ToolTip.text

                background: Rectangle {
                    radius: 4
                    color: starButton.hovered ? Theme.buttonGhostHover : Theme.transparent
                }

                contentItem: EchoIcon {
                    source: toolbar.asset !== null && starButton.ratingValue <= toolbar.asset.rating ? "qrc:/EchoDesktop/icons/star-filled.svg" : "qrc:/EchoDesktop/icons/star.svg"
                    color: toolbar.asset !== null && starButton.ratingValue <= toolbar.asset.rating ? Theme.ratingAccent : Theme.textDisabled
                    size: 14
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        EchoButton {
            text: qsTr("Assemble")
            ghost: true
            onClicked: toolbar.assembleRequested()
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

            contentItem: EchoIcon {
                source: "qrc:/EchoDesktop/icons/album-add.svg"
                color: Theme.textSecondary
                size: 16
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
                checked: toolbar.asset !== null && modelData.memberIds.includes(toolbar.asset.id)
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
