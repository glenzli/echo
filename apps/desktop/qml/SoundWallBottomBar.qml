//! Sound Wall browse state. Sort, additive metadata facets, result count and
//! semantic card density are global to the wall, so they live in one bottom
//! toolbar instead of competing with window navigation in the title bar.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

ToolBar {
    id: bar

    required property string sortMode
    required property bool likedOnly
    required property int minimumRating
    required property bool textOnly
    required property int visibleCount
    required property int totalCount
    required property real cardWidth

    signal sortRequested(string mode)
    signal likedFilterRequested(bool enabled)
    signal ratingFilterRequested(int minimumRating)
    signal textFilterRequested(bool enabled)
    signal cardWidthRequested(real width)

    readonly property string densityLabel: cardWidth <= 260
        ? qsTr("Overview") : cardWidth <= 360 ? qsTr("Browse") : qsTr("Rich")

    implicitHeight: 42
    topPadding: 5
    bottomPadding: 5
    leftPadding: 12
    rightPadding: 12

    background: Rectangle {
        color: Theme.chrome

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 1
            color: Theme.border
        }
    }

    contentItem: RowLayout {
        spacing: 8

        ComboBox {
            id: sortBox

            Layout.preferredWidth: 96
            Layout.preferredHeight: Theme.compactControlHeight
            model: [qsTr("Date"), qsTr("Duration"), qsTr("Rating")]
            currentIndex: Math.max(0, ["date", "duration", "rating"]
                .indexOf(bar.sortMode))
            onActivated: index => bar.sortRequested(
                ["date", "duration", "rating"][index])

            ToolTip.visible: hovered
            ToolTip.text: qsTr("Sort sounds")
            ToolTip.delay: 600

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
                border.color: sortBox.activeFocus ? Theme.accent : Theme.buttonBorder
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        Button {
            id: likedFilter

            Layout.preferredWidth: 30
            Layout.preferredHeight: 30
            padding: 0
            focusPolicy: Qt.NoFocus
            onClicked: bar.likedFilterRequested(!bar.likedOnly)

            ToolTip.visible: hovered
            ToolTip.text: qsTr("Show liked sounds only")
            ToolTip.delay: 600
            Accessible.name: ToolTip.text

            background: Rectangle {
                radius: Theme.compactControlRadius
                border.width: bar.likedOnly ? 1 : 0
                border.color: Theme.accent
                color: bar.likedOnly ? Theme.accentSurfaceQuiet
                    : likedFilter.hovered ? Theme.buttonGhostHover : Theme.transparent
            }

            contentItem: Text {
                text: bar.likedOnly ? "♥" : "♡"
                color: bar.likedOnly ? "#dc4b6b" : Theme.textSecondary
                font.pixelSize: 16
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        Row {
            spacing: 0

            Repeater {
                model: 5

                delegate: Button {
                    id: ratingFilter
                    required property int index

                    readonly property int ratingValue: index + 1
                    implicitWidth: 20
                    implicitHeight: 30
                    padding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: bar.ratingFilterRequested(
                        bar.minimumRating === ratingValue ? 0 : ratingValue)

                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("At least %1 stars").arg(ratingValue)
                    ToolTip.delay: 600
                    Accessible.name: ToolTip.text

                    background: Rectangle {
                        radius: 4
                        color: ratingFilter.hovered
                            ? Theme.buttonGhostHover : Theme.transparent
                    }

                    contentItem: Text {
                        text: ratingFilter.ratingValue <= bar.minimumRating ? "★" : "☆"
                        color: ratingFilter.ratingValue <= bar.minimumRating
                            ? "#d89a16" : Theme.textDisabled
                        font.pixelSize: 14
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

        Button {
            id: textFilter

            Layout.preferredWidth: 52
            Layout.preferredHeight: 30
            padding: 0
            focusPolicy: Qt.NoFocus
            onClicked: bar.textFilterRequested(!bar.textOnly)

            ToolTip.visible: hovered
            ToolTip.text: qsTr("Show sounds with text only")
            ToolTip.delay: 600
            Accessible.name: ToolTip.text

            background: Rectangle {
                radius: Theme.compactControlRadius
                border.width: bar.textOnly ? 1 : 0
                border.color: Theme.accent
                color: bar.textOnly ? Theme.accentSurfaceQuiet
                    : textFilter.hovered ? Theme.buttonGhostHover : Theme.transparent
            }

            contentItem: Text {
                text: qsTr("Text")
                color: bar.textOnly ? Theme.accentSelectionText : Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                font.bold: bar.textOnly
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        Item { Layout.fillWidth: true }

        Text {
            text: qsTr("%1 / %2 sounds").arg(bar.visibleCount).arg(bar.totalCount)
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        Text {
            text: bar.densityLabel
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            Layout.minimumWidth: 48
            horizontalAlignment: Text.AlignRight
        }

        Text {
            text: "▦"
            color: Theme.textDisabled
            font.pixelSize: 13
        }

        Slider {
            id: densitySlider

            Layout.preferredWidth: 112
            from: 220
            to: 460
            stepSize: 10
            value: bar.cardWidth
            onMoved: bar.cardWidthRequested(value)

            ToolTip.visible: hovered || pressed
            ToolTip.text: qsTr("Card detail: %1").arg(bar.densityLabel)
            ToolTip.delay: 400
            Accessible.name: qsTr("Sound card detail")
        }

        Text {
            text: "▤"
            color: Theme.textSecondary
            font.pixelSize: 14
        }
    }
}
