//! Shadow-series presentation controls for the Audio Space center surface.
//! Collection identity, content search and presentation scale live above the
//! surface they affect; sorting and evidence filters remain in the bottom bar.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

ToolBar {
    id: toolbar

    required property string collectionName
    required property int visibleCount
    required property string viewMode
    required property real cardWidth
    required property string searchText
    required property bool semanticSearching
    required property bool revisitMode

    signal searchRequested(string text)
    signal viewModeRequested(string mode)
    signal cardWidthRequested(real width)
    signal processingRecipesRequested
    signal processingRecipeManagementRequested
    signal processingHistoryRequested
    signal batchExportRequested

    implicitHeight: 46
    leftPadding: 14
    rightPadding: 12
    topPadding: 6
    bottomPadding: 6

    background: Rectangle {
        color: Theme.chrome

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }
    }

    contentItem: RowLayout {
        spacing: 9

        Text {
            Layout.maximumWidth: 150
            text: toolbar.collectionName
            color: Theme.textPrimary
            font.pixelSize: 12
            font.bold: true
            elide: Text.ElideRight
        }

        Rectangle {
            Layout.preferredWidth: visibleCountText.implicitWidth + 12
            Layout.preferredHeight: 22
            radius: 7
            color: Theme.surfaceSubtle

            Text {
                id: visibleCountText
                anchors.centerIn: parent
                text: toolbar.revisitMode ? qsTr("%1 memories").arg(toolbar.visibleCount) : qsTr("%1 visible").arg(toolbar.visibleCount)
                color: Theme.textMuted
                font.pixelSize: 9
                font.bold: true
            }
        }

        EchoTextField {
            Layout.leftMargin: 4
            Layout.minimumWidth: 150
            Layout.preferredWidth: 240
            Layout.maximumWidth: 300
            implicitHeight: Theme.compactControlHeight
            text: toolbar.searchText
            placeholderText: qsTr("Search sounds by words or meaning…")
            onTextEdited: toolbar.searchRequested(text)
        }

        BusyIndicator {
            visible: toolbar.semanticSearching
            running: visible
            Layout.preferredWidth: 18
            Layout.preferredHeight: 18
            Accessible.name: qsTr("Searching meaning")
        }

        Item {
            Layout.fillWidth: true
        }

        Rectangle {
            visible: !toolbar.revisitMode
            Layout.preferredWidth: processingActions.implicitWidth + 6
            Layout.preferredHeight: 32
            radius: 8
            color: Theme.surfaceSubtle

            RowLayout {
                id: processingActions
                anchors.centerIn: parent
                spacing: 0

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/tune.svg"
                    enabled: toolbar.visibleCount > 0
                    toolTipText: qsTr("Apply processing recipe")
                    accessibleName: toolTipText
                    buttonSize: 28
                    iconSize: 15
                    onClicked: toolbar.processingRecipesRequested()
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/equalizer.svg"
                    toolTipText: qsTr("Manage processing recipes")
                    accessibleName: toolTipText
                    buttonSize: 28
                    iconSize: 15
                    onClicked: toolbar.processingRecipeManagementRequested()
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/history.svg"
                    toolTipText: qsTr("Processing history")
                    accessibleName: toolTipText
                    buttonSize: 28
                    iconSize: 15
                    onClicked: toolbar.processingHistoryRequested()
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/export.svg"
                    enabled: toolbar.visibleCount > 0 && !batchExporter.running
                    selected: batchExporter.running || batchExporter.recoverable
                    toolTipText: batchExporter.recoverable ? qsTr("Resume batch export") : qsTr("Export current results")
                    accessibleName: toolTipText
                    buttonSize: 28
                    iconSize: 15
                    onClicked: toolbar.batchExportRequested()
                }
            }
        }

        Rectangle {
            visible: !toolbar.revisitMode
            Layout.preferredWidth: viewActions.implicitWidth + 6
            Layout.preferredHeight: 32
            radius: 8
            color: Theme.surfaceSubtle

            RowLayout {
                id: viewActions
                anchors.centerIn: parent
                spacing: 0

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/review-grid.svg"
                    selected: toolbar.viewMode === "grid"
                    toolTipText: qsTr("Grid view")
                    accessibleName: toolTipText
                    buttonSize: 28
                    iconSize: 15
                    onClicked: toolbar.viewModeRequested("grid")
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/tape.svg"
                    selected: toolbar.viewMode === "tape"
                    toolTipText: qsTr("Sound tape")
                    accessibleName: toolTipText
                    buttonSize: 28
                    iconSize: 15
                    onClicked: toolbar.viewModeRequested("tape")
                }
            }
        }

        Item {
            visible: toolbar.viewMode === "grid" && !toolbar.revisitMode
            Layout.preferredWidth: 154
            Layout.preferredHeight: 28

            RowLayout {
                anchors.fill: parent
                spacing: 7

                EchoIcon {
                    source: "qrc:/EchoDesktop/icons/waveform.svg"
                    size: 14
                    color: Theme.textMuted
                }

                Slider {
                    id: cardScaleSlider

                    Layout.fillWidth: true
                    Layout.preferredHeight: 26
                    from: 180
                    to: 420
                    stepSize: 8
                    value: toolbar.cardWidth
                    onMoved: toolbar.cardWidthRequested(value)

                    ToolTip.visible: hovered || pressed
                    ToolTip.text: qsTr("Sound card size")
                    ToolTip.delay: 400
                    Accessible.name: qsTr("SCALE")

                    background: Rectangle {
                        x: cardScaleSlider.leftPadding
                        y: Math.round((cardScaleSlider.height - height) / 2)
                        width: cardScaleSlider.availableWidth
                        height: 3
                        radius: 2
                        color: Theme.track
                    }

                    handle: Rectangle {
                        x: cardScaleSlider.leftPadding + cardScaleSlider.visualPosition * (cardScaleSlider.availableWidth - width)
                        y: Math.round((cardScaleSlider.height - height) / 2)
                        implicitWidth: cardScaleSlider.pressed ? 13 : 11
                        implicitHeight: cardScaleSlider.pressed ? 13 : 11
                        radius: width / 2
                        color: Theme.panelRaised
                        border.width: 1
                        border.color: Theme.accent

                        Behavior on implicitWidth {
                            NumberAnimation {
                                duration: 80
                            }
                        }
                        Behavior on implicitHeight {
                            NumberAnimation {
                                duration: 80
                            }
                        }
                    }
                }
            }
        }

        Rectangle {
            visible: toolbar.revisitMode
            Layout.preferredWidth: 32
            Layout.preferredHeight: 32
            radius: 8
            color: Theme.surfaceSelected

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/history.svg"
                size: 16
                color: Theme.accent
            }

            ToolTip.visible: revisitHover.hovered
            ToolTip.text: qsTr("Revisit home")

            HoverHandler {
                id: revisitHover
            }
        }
    }
}
