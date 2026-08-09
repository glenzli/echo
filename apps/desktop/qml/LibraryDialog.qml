//! Transient library-management surface launched from the application
//! toolbar. The trigger remains in Main while LibraryPanel owns its models.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(840, Math.max(680, parent.width - 96))
    height: Math.min(650, Math.max(520, parent.height - 96))
    title: qsTr("Library")
    modal: true
    dim: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    Overlay.modal: Rectangle {
        color: Theme.effectiveDark ? "#99000000" : "#660f1720"
    }

    background: Rectangle {
        color: Theme.panelRaised
        radius: 14
        border.color: Theme.borderStrong
        border.width: 1
    }

    header: Rectangle {
        implicitHeight: 48
        color: Theme.chrome
        radius: 14

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 15
            color: parent.color
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            text: qsTr("Audio library")
            color: Theme.textPrimary
            font.pixelSize: 14
            font.bold: true
        }
    }

    footer: Rectangle {
        implicitHeight: 52
        color: Theme.chrome
        radius: 14

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 15
            color: parent.color
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 1
            color: Theme.border
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 14

            Text {
                text: qsTr("Changes are applied immediately")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Done")
                onClicked: dialog.close()
            }
        }
    }

    contentItem: Rectangle {
        color: Theme.window

        LibraryPanel {
            anchors.fill: parent
            anchors.margins: 22
        }
    }
}
