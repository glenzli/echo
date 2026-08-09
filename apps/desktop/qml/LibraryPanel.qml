//! LibraryPanel: the library management workspace — scan roots, missing
//! assets, and cache health. Own surface, styled with Echo components.

import QtQuick
import QtQuick.Layouts
import EchoDesktop

ColumnLayout {
    id: panel

    spacing: 16

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        Text {
            text: qsTr("Library")
            color: Theme.textPrimary
            font.pixelSize: 22
            font.bold: true
        }

        Text {
            text: qsTr("Folders Echo watches and listens to")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
        }

        Item { Layout.fillWidth: true }

        EchoIconButton {
            source: "qrc:/EchoDesktop/icons/refresh.svg"
            toolTipText: qsTr("Refresh library")
            onClicked: {
                backend.queueScans()
                rootModel.refresh()
                missingModel.refresh()
            }
        }
    }

    EchoSectionLabel {
        Layout.fillWidth: true
        text: qsTr("Scan roots")
        hint: qsTr("New and changed recordings are imported and prepared for browsing "
                   + "in the background. Missing files re-link automatically when "
                   + "they return.")
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(180, rootModel.count * 36 + 16)
        color: Theme.panel
        radius: 10
        border.color: Theme.border

        ListView {
            anchors.fill: parent
            anchors.margins: 8
            spacing: 4
            clip: true
            model: rootModel

            delegate: Rectangle {
                required property var modelData

                width: parent ? parent.width : 0
                height: 32
                radius: Theme.compactControlRadius
                color: Theme.panelRaised

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 6
                    spacing: 8

                    Text {
                        text: modelData.root
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }

                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/stop.svg"
                        toolTipText: qsTr("Remove")
                        buttonSize: 26
                        iconSize: 14
                        onClicked: backend.removeRoot(modelData.id)
                    }
                }
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        EchoTextField {
            id: rootField

            Layout.fillWidth: true
            placeholderText: qsTr("/path/to/recordings")
        }

        EchoButton {
            text: qsTr("Add")
            onClicked: {
                if (rootField.text.length > 0) {
                    backend.addRoot(rootField.text)
                    rootField.clear()
                    rootModel.refresh()
                }
            }
        }
    }

    EchoSectionLabel {
        Layout.fillWidth: true
        text: qsTr("Missing recordings")
        hint: qsTr("Evidence stays in the library; the file re-links "
                   + "automatically when it returns.")
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: missingModel.count > 0
            ? Math.min(120, missingModel.count * 32 + 16) : 40
        color: Theme.panel
        radius: 10
        border.color: Theme.border

        ListView {
            anchors.fill: parent
            anchors.margins: 8
            spacing: 2
            clip: true
            model: missingModel

            delegate: Rectangle {
                required property var modelData

                width: parent ? parent.width : 0
                height: 28
                radius: Theme.compactControlRadius
                color: Theme.transparent

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    spacing: 8

                    Rectangle {
                        Layout.preferredWidth: 54
                        Layout.preferredHeight: 16
                        radius: 8
                        color: Theme.accentSurfaceQuiet

                        Text {
                            anchors.centerIn: parent
                            text: qsTr("Missing")
                            color: Theme.accentSelectionText
                            font.pixelSize: Theme.fontMeta
                        }
                    }

                    Text {
                        text: modelData.path
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontBody
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }

    EchoSectionLabel {
        Layout.fillWidth: true
        text: qsTr("Storage")
        hint: qsTr("The cache holds rebuildable waveform and analysis data.")
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 44
        color: Theme.panel
        radius: 10
        border.color: Theme.border

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 8

            Text {
                text: qsTr("Cache")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
            }

            Text {
                text: backend.cacheRoot
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                elide: Text.ElideMiddle
                Layout.fillWidth: true
            }
        }
    }

    ListModel {
        id: rootModel

        function refresh() : void {
            clear()
            const roots = backend.listRoots()
            for (const root of roots) {
                append(root)
            }
        }
    }

    Connections {
        target: backend
        function onAssetsChanged() : void {
            rootModel.refresh()
            missingModel.refresh()
        }
    }

    ListModel {
        id: missingModel

        function refresh() : void {
            clear()
            const assets = backend.listAssets()
            for (const asset of assets) {
                if (asset.pathStatus === "missing") {
                    append(asset)
                }
            }
        }
    }

    Component.onCompleted: {
        rootModel.refresh()
        missingModel.refresh()
    }
}
