//! Global Audio Space sorting and evidence filters. Presentation and card
//! scale belong to SoundPresentationToolbar above the center surface.

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
    required property var filterState

    signal sortRequested(string mode)
    signal likedFilterRequested(bool enabled)
    signal ratingFilterRequested(int minimumRating)
    signal textFilterRequested(bool enabled)
    signal clearAllFiltersRequested()

    readonly property bool anyFilterActive: likedOnly || minimumRating > 0
        || textOnly || filterState.activeCount > 0

    implicitHeight: 44
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

    SoundAdvancedFilterPopup {
        id: advancedFilters
        filterState: bar.filterState
    }

    contentItem: Item {
        Rectangle {
            id: filterCapsule

            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            width: filterControls.implicitWidth + 10
            height: 32
            radius: 8
            color: bar.anyFilterActive ? Theme.accentSurfaceQuiet
                                       : Theme.surfaceSubtle

            RowLayout {
                id: filterControls

                anchors.centerIn: parent
                height: 28
                spacing: 2

                ComboBox {
                    id: sortBox

                    Layout.preferredWidth: 82
                    Layout.preferredHeight: 26
                    model: [qsTr("Date"), qsTr("Duration"), qsTr("Rating")]
                    currentIndex: Math.max(0, ["date", "duration", "rating"]
                        .indexOf(bar.sortMode))
                    onActivated: index => bar.sortRequested(
                        ["date", "duration", "rating"][index])

                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Sort sounds")
                    ToolTip.delay: 600

                    contentItem: Text {
                        leftPadding: 8
                        rightPadding: 20
                        text: sortBox.displayText
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontMeta
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    indicator: Text {
                        x: sortBox.width - width - 7
                        y: Math.round((sortBox.height - height) / 2) - 1
                        text: "⌄"
                        color: Theme.textSecondary
                        font.pixelSize: 12
                    }

                    background: Rectangle {
                        radius: 6
                        color: sortBox.down ? Theme.controlPressed : Theme.control
                        border.width: 0
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 16
                    color: Theme.border
                }

                EchoIconButton {
                    id: advancedFilterButton
                    source: "qrc:/EchoDesktop/icons/filter.svg"
                    selected: bar.filterState.activeCount > 0
                    toolTipText: qsTr("Open advanced filters")
                    buttonSize: 26
                    iconSize: 14
                    onClicked: advancedFilters.presentFrom(advancedFilterButton)
                }

                Text {
                    text: bar.filterState.activeCount > 0
                        ? qsTr("FILTER %1").arg(bar.filterState.activeCount)
                        : qsTr("FILTER")
                    color: bar.filterState.activeCount > 0
                        ? Theme.accentSelectionText : Theme.textSecondary
                    font.pixelSize: 9
                    font.bold: bar.filterState.activeCount > 0
                    font.letterSpacing: 0.6
                    verticalAlignment: Text.AlignVCenter

                    TapHandler {
                        onTapped: advancedFilters.presentFrom(advancedFilterButton)
                    }
                }

                EchoIconButton {
                    visible: bar.anyFilterActive
                    source: "qrc:/EchoDesktop/icons/filter-off.svg"
                    toolTipText: qsTr("Clear all filters")
                    buttonSize: 26
                    iconSize: 14
                    onClicked: bar.clearAllFiltersRequested()
                }

                Rectangle {
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 16
                    color: Theme.border
                }

                Button {
                    id: likedFilter

                    Layout.preferredWidth: 26
                    Layout.preferredHeight: 26
                    padding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: bar.likedFilterRequested(!bar.likedOnly)

                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Show liked sounds only")
                    ToolTip.delay: 600
                    Accessible.name: ToolTip.text

                    background: Rectangle {
                        radius: 6
                        color: bar.likedOnly ? Theme.accentSurface
                            : likedFilter.hovered ? Theme.buttonGhostHover
                                                  : Theme.transparent
                    }

                    contentItem: Text {
                        text: bar.likedOnly ? "♥" : "♡"
                        color: bar.likedOnly ? "#dc4b6b" : Theme.textSecondary
                        font.pixelSize: 15
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                Row {
                    Layout.alignment: Qt.AlignVCenter
                    spacing: 0

                    Repeater {
                        model: 5

                        delegate: Button {
                            id: ratingFilter
                            required property int index

                            readonly property int ratingValue: index + 1
                            width: 18
                            height: 26
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
                                text: ratingFilter.ratingValue <= bar.minimumRating
                                    ? "★" : "☆"
                                color: ratingFilter.ratingValue <= bar.minimumRating
                                    ? "#d89a16" : Theme.textDisabled
                                font.pixelSize: 13
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                    }
                }

                Button {
                    id: textFilter

                    Layout.preferredWidth: 40
                    Layout.preferredHeight: 26
                    padding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: bar.textFilterRequested(!bar.textOnly)

                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Show sounds with text only")
                    ToolTip.delay: 600
                    Accessible.name: ToolTip.text

                    background: Rectangle {
                        radius: 6
                        color: bar.textOnly ? Theme.accentSurface
                            : textFilter.hovered ? Theme.buttonGhostHover
                                                 : Theme.transparent
                    }

                    contentItem: Text {
                        text: qsTr("Text")
                        color: bar.textOnly ? Theme.accentSelectionText
                                            : Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        font.bold: bar.textOnly
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

    }
}
