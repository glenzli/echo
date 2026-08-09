//! Transient library-management surface launched from the application
//! toolbar. The trigger remains in Main while LibraryPanel owns its models.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    title: qsTr("Library")
    modal: true
    width: 840
    height: 650
    padding: 0

    header: Rectangle {
        implicitHeight: 48
        color: Theme.chrome

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
