//! Contextual operations for the selected sound. Cards expose affinity state
//! but do not mutate it; the selected identity owns Like, rating and expansion.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: toolbar

    required property var asset

    signal affinityRequested(bool liked, int rating)
    signal expandRequested()

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

        EchoButton {
            text: qsTr("Expand") + " ↗"
            ghost: true
            implicitWidth: 78
            implicitHeight: 30
            onClicked: toolbar.expandRequested()

            ToolTip.visible: hovered
            ToolTip.text: qsTr("Open the selected sound")
            ToolTip.delay: 500
        }
    }
}
