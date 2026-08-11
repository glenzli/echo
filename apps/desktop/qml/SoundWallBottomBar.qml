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
    required property bool speechOnly
    required property string analysisFilter
    required property int incompleteAnalysisCount
    required property int failedAnalysisCount
    required property int manualAnalysisCount
    required property var filterState

    signal sortRequested(string mode)
    signal likedFilterRequested(bool enabled)
    signal ratingFilterRequested(int minimumRating)
    signal speechFilterRequested(bool enabled)
    signal analysisFilterRequested(string filter)
    signal retryFailedAnalysisRequested
    signal clearAllFiltersRequested

    readonly property bool anyFilterActive: likedOnly || minimumRating > 0 || speechOnly || analysisFilter !== "all" || filterState.activeCount > 0

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
            color: bar.anyFilterActive ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle

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
                    currentIndex: Math.max(0, ["date", "duration", "rating"].indexOf(bar.sortMode))
                    onActivated: index => bar.sortRequested(["date", "duration", "rating"][index])

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

                    indicator: EchoIcon {
                        x: sortBox.width - width - 7
                        y: Math.round((sortBox.height - height) / 2) - 1
                        source: "qrc:/EchoDesktop/icons/chevron-down.svg"
                        color: Theme.textSecondary
                        size: 12
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

                Item {
                    Layout.preferredWidth: 28
                    Layout.preferredHeight: 28

                    EchoIconButton {
                        id: advancedFilterButton
                        anchors.centerIn: parent
                        source: "qrc:/EchoDesktop/icons/filter.svg"
                        selected: bar.filterState.activeCount > 0
                        toolTipText: qsTr("Open advanced filters")
                        buttonSize: 26
                        iconSize: 14
                        onClicked: advancedFilters.presentFrom(advancedFilterButton)
                    }

                    Rectangle {
                        visible: bar.filterState.activeCount > 0
                        anchors.right: parent.right
                        anchors.top: parent.top
                        width: Math.max(13, filterCount.implicitWidth + 6)
                        height: 13
                        radius: height / 2
                        color: Theme.accent
                        z: 3

                        Text {
                            id: filterCount
                            anchors.centerIn: parent
                            text: bar.filterState.activeCount
                            color: Theme.accentSelectionText
                            font.pixelSize: 8
                            font.bold: true
                        }
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
                        color: bar.likedOnly ? Theme.likeSurface : likedFilter.hovered ? Theme.buttonGhostHover : Theme.transparent
                    }

                    contentItem: EchoIcon {
                        source: bar.likedOnly ? "qrc:/EchoDesktop/icons/heart-filled.svg" : "qrc:/EchoDesktop/icons/heart.svg"
                        color: bar.likedOnly ? Theme.likeAccent : Theme.textSecondary
                        size: 15
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
                            onClicked: bar.ratingFilterRequested(bar.minimumRating === ratingValue ? 0 : ratingValue)

                            ToolTip.visible: hovered
                            ToolTip.text: qsTr("At least %1 stars").arg(ratingValue)
                            ToolTip.delay: 600
                            Accessible.name: ToolTip.text

                            background: Rectangle {
                                radius: 4
                                color: ratingFilter.hovered ? Theme.buttonGhostHover : Theme.transparent
                            }

                            contentItem: EchoIcon {
                                source: ratingFilter.ratingValue <= bar.minimumRating ? "qrc:/EchoDesktop/icons/star-filled.svg" : "qrc:/EchoDesktop/icons/star.svg"
                                color: ratingFilter.ratingValue <= bar.minimumRating ? Theme.ratingAccent : Theme.textDisabled
                                size: 13
                            }
                        }
                    }
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/mic.svg"
                    selected: bar.speechOnly
                    toolTipText: qsTr("Show sounds with speech only")
                    buttonSize: 26
                    iconSize: 14
                    onClicked: bar.speechFilterRequested(!bar.speechOnly)
                }
            }
        }

        Rectangle {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: analysisControls.implicitWidth + 10
            height: 32
            radius: 8
            color: bar.analysisFilter !== "all" ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle

            RowLayout {
                id: analysisControls

                anchors.centerIn: parent
                height: 28
                spacing: 2

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/clock.svg"
                    selected: bar.analysisFilter === "incomplete"
                    toolTipText: qsTr("Show sounds that still need analysis")
                    buttonSize: 26
                    iconSize: 14
                    onClicked: bar.analysisFilterRequested("incomplete")
                }

                Text {
                    visible: bar.incompleteAnalysisCount > 0
                    text: bar.incompleteAnalysisCount
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/refresh.svg"
                    selected: bar.analysisFilter === "failed"
                    toolTipText: qsTr("Show analysis failures")
                    buttonSize: 26
                    iconSize: 14
                    onClicked: bar.analysisFilterRequested("failed")
                }

                Text {
                    visible: bar.failedAnalysisCount > 0
                    text: bar.failedAnalysisCount
                    color: bar.failedAnalysisCount > 0 ? Theme.warningText : Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                Rectangle {
                    visible: bar.manualAnalysisCount > 0
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 16
                    color: Theme.border
                }

                EchoIconButton {
                    visible: bar.manualAnalysisCount > 0
                    source: "qrc:/EchoDesktop/icons/redo.svg"
                    toolTipText: qsTr("Retry all failed analysis")
                    buttonSize: 26
                    iconSize: 14
                    onClicked: bar.retryFailedAnalysisRequested()
                }
            }
        }
    }
}
