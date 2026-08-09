//! Anchored advanced filters for existing Library evidence. The trigger owns
//! this popup, while SoundFilterState owns selection and matching semantics.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: popup

    required property var filterState

    parent: Overlay.overlay
    width: Math.min(560, parent.width - 24)
    height: Math.min(520, parent.height - 24)
    padding: 12
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function presentFrom(item: var) : void {
        const point = item.mapToItem(Overlay.overlay, 0, 0)
        x = Math.max(12, Math.min(parent.width - width - 12,
                                 point.x + item.width / 2 - width / 2))
        y = Math.max(12, point.y - height - 8)
        open()
    }

    background: Rectangle {
        radius: 10
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    component FacetSection: ColumnLayout {
        id: section

        required property string title
        required property string facet
        required property var options
        required property var selectedKeys

        signal toggled(string facet, string key)

        Layout.fillWidth: true
        spacing: 6

        Text {
            Layout.fillWidth: true
            text: section.title.toUpperCase()
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            font.bold: true
            font.letterSpacing: 0.8
        }

        Flow {
            Layout.fillWidth: true
            spacing: 5

            Repeater {
                model: section.options

                delegate: Button {
                    id: chip
                    required property var modelData

                    readonly property bool selected:
                        section.selectedKeys.includes(String(modelData.key))

                    implicitWidth: chipLabel.implicitWidth + 16
                    implicitHeight: 26
                    padding: 0
                    focusPolicy: Qt.NoFocus
                    onClicked: section.toggled(section.facet, String(modelData.key))

                    background: Rectangle {
                        radius: 7
                        border.width: chip.selected ? 1 : 0
                        border.color: Theme.accent
                        color: chip.selected ? Theme.accentSurfaceQuiet
                            : chip.hovered ? Theme.buttonGhostHover : Theme.surfaceSubtle
                    }

                    contentItem: Text {
                        id: chipLabel
                        text: String(chip.modelData.label) + "  " + chip.modelData.count
                        color: chip.selected ? Theme.accentSelectionText
                                             : Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: section.options.length === 0
            text: qsTr("No values in this Library")
            color: Theme.textDisabled
            font.pixelSize: Theme.fontMeta
        }
    }

    contentItem: ColumnLayout {
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Advanced filters")
                    color: Theme.textPrimary
                    font.pixelSize: 14
                    font.bold: true
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Match any value within a category and every active category.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }
            }

            EchoButton {
                text: qsTr("Clear all")
                ghost: true
                enabled: popup.filterState.activeCount > 0
                implicitHeight: 28
                onClicked: popup.filterState.clear()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/filter-off.svg"
                toolTipText: qsTr("Close advanced filters")
                buttonSize: 28
                iconSize: 14
                onClicked: popup.close()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: parent.width
                spacing: 13

                FacetSection {
                    title: qsTr("Keywords")
                    facet: "keyword"
                    options: popup.filterState.keywordOptions
                    selectedKeys: popup.filterState.selectedKeywords
                    onToggled: (facet, key) => popup.filterState.toggle(facet, key)
                }

                FacetSection {
                    title: qsTr("Mood")
                    facet: "mood"
                    options: popup.filterState.moodOptions
                    selectedKeys: popup.filterState.selectedMoods
                    onToggled: (facet, key) => popup.filterState.toggle(facet, key)
                }

                FacetSection {
                    title: qsTr("Recording year")
                    facet: "year"
                    options: popup.filterState.yearOptions
                    selectedKeys: popup.filterState.selectedYears
                    onToggled: (facet, key) => popup.filterState.toggle(facet, key)
                }

                FacetSection {
                    title: qsTr("Location")
                    facet: "location"
                    options: popup.filterState.locationOptions
                    selectedKeys: popup.filterState.selectedLocations
                    onToggled: (facet, key) => popup.filterState.toggle(facet, key)
                }

                FacetSection {
                    title: qsTr("Events")
                    facet: "event"
                    options: popup.filterState.eventOptions
                    selectedKeys: popup.filterState.selectedEvents
                    onToggled: (facet, key) => popup.filterState.toggle(facet, key)
                }
            }
        }
    }
}
