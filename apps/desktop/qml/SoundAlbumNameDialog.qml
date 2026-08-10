//! Centered name editor shared by album creation, rename, and suggestion
//! confirmation. User text crosses persistence only on explicit confirmation.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    property string heading: ""
    property string actionText: ""
    property string initialName: ""

    signal submitted(string name)

    function show(title: string, action: string, name: string) : void {
        heading = title
        actionText = action
        initialName = name
        nameField.text = name
        validationText.text = ""
        open()
        nameField.forceActiveFocus()
        nameField.selectAll()
    }

    parent: Overlay.overlay
    modal: true
    dim: true
    width: 380
    height: 202
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    padding: 0

    background: Rectangle {
        radius: 12
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    contentItem: ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 12

        Text {
            Layout.fillWidth: true
            text: dialog.heading
            color: Theme.textPrimary
            font.pixelSize: 16
            font.bold: true
        }

        EchoTextField {
            id: nameField

            Layout.fillWidth: true
            placeholderText: qsTr("Album name")
            maximumLength: 80
            onAccepted: confirmButton.clicked()
            onTextChanged: validationText.text = ""
        }

        Text {
            id: validationText

            Layout.fillWidth: true
            Layout.preferredHeight: 16
            color: Theme.warningText
            font.pixelSize: Theme.fontMeta
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Cancel")
                ghost: true
                onClicked: dialog.close()
            }

            EchoButton {
                id: confirmButton

                text: dialog.actionText
                enabled: nameField.text.trim().length > 0
                onClicked: {
                    const name = nameField.text.trim()
                    if (name.length === 0) {
                        validationText.text = qsTr("Enter an album name.")
                        return
                    }
                    dialog.submitted(name)
                    dialog.close()
                }
            }
        }
    }
}
