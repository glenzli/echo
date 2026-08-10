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

    signal searchRequested(string text)
    signal viewModeRequested(string mode)
    signal cardWidthRequested(real width)

    implicitHeight: 44
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
        spacing: 8

        Text {
            text: toolbar.collectionName.toUpperCase()
            color: Theme.textSecondary
            font.pixelSize: 9
            font.bold: true
            font.letterSpacing: 0.8
            elide: Text.ElideRight
        }

        Text {
            text: qsTr("%1 visible").arg(toolbar.visibleCount)
            color: Theme.textPrimary
            font.pixelSize: Theme.fontMeta
        }

        EchoTextField {
            Layout.leftMargin: 8
            Layout.minimumWidth: 140
            Layout.preferredWidth: 230
            Layout.maximumWidth: 280
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

        Item { Layout.fillWidth: true }

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
            source: "qrc:/EchoDesktop/icons/filmstrip.svg"
            selected: toolbar.viewMode === "focus"
            toolTipText: qsTr("Single sound with filmstrip")
            accessibleName: toolTipText
            buttonSize: 28
            iconSize: 15
            onClicked: toolbar.viewModeRequested("focus")
        }

        Text {
            visible: toolbar.viewMode === "grid"
            text: qsTr("SCALE")
            color: Theme.textSecondary
            font.pixelSize: 9
            font.bold: true
            font.letterSpacing: 0.7
        }

        Slider {
            id: cardScaleSlider

            visible: toolbar.viewMode === "grid"
            Layout.preferredWidth: 138
            Layout.preferredHeight: 28
            from: 180
            to: 420
            stepSize: 8
            value: toolbar.cardWidth
            onMoved: toolbar.cardWidthRequested(value)

            ToolTip.visible: hovered || pressed
            ToolTip.text: qsTr("Sound card size")
            ToolTip.delay: 400
            Accessible.name: ToolTip.text
        }
    }
}
