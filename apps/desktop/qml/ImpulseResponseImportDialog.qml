pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    modal: true
    anchors.centerIn: Overlay.overlay
    width: 520
    title: qsTr("Import impulse response")
    standardButtons: Dialog.Cancel

    function present(): void {
        sourcePath.text = "";
        displayName.text = "";
        creator.text = "";
        sourceUrl.text = "";
        attribution.text = "";
        spdx.text = "CC0-1.0";
        rights.currentIndex = 0;
        open();
        wavPicker.open();
    }

    FileDialog {
        id: wavPicker
        title: qsTr("Choose a local WAV impulse response")
        fileMode: FileDialog.OpenFile
        nameFilters: [qsTr("WAV audio (*.wav)")]
        onAccepted: {
            sourcePath.text = selectedFile.toLocalFile();
            if (displayName.text.length === 0)
                displayName.text = selectedFile.toString().split("/").pop().replace(/\.wav$/i, "");
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 10

        Text {
            Layout.fillWidth: true
            text: qsTr("Echo preserves the original WAV, a canonical prepared copy, and the rights declaration as separate evidence.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }

        TextField {
            id: sourcePath
            Layout.fillWidth: true
            readOnly: true
            placeholderText: qsTr("Local WAV file")
        }
        TextField {
            id: displayName
            Layout.fillWidth: true
            placeholderText: qsTr("Display name")
        }
        TextField {
            id: creator
            Layout.fillWidth: true
            placeholderText: qsTr("Creator (optional)")
        }
        TextField {
            id: sourceUrl
            Layout.fillWidth: true
            placeholderText: qsTr("Source URL (optional)")
        }
        TextField {
            id: attribution
            Layout.fillWidth: true
            placeholderText: qsTr("Attribution (optional)")
        }

        ComboBox {
            id: rights
            Layout.fillWidth: true
            model: [qsTr("SPDX or public licence"), qsTr("My recording · no redistribution")]
        }

        TextField {
            id: spdx
            Layout.fillWidth: true
            visible: rights.currentIndex === 0
            placeholderText: qsTr("SPDX expression, for example CC0-1.0")
        }

        Text {
            Layout.fillWidth: true
            visible: impulseResponseController.errorText.length > 0
            text: impulseResponseController.errorText
            color: Theme.warningText
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight

            Button {
                text: qsTr("Choose another file")
                enabled: !impulseResponseController.busy
                onClicked: wavPicker.open()
            }
            Button {
                text: impulseResponseController.busy ? qsTr("Importing…") : qsTr("Import")
                enabled: !impulseResponseController.busy && sourcePath.text.length > 0 && displayName.text.trim().length > 0 && (rights.currentIndex === 1 || spdx.text.trim().length > 0)
                onClicked: impulseResponseController.importLocalWav(sourcePath.text, displayName.text, creator.text, sourceUrl.text, attribution.text, rights.currentIndex === 0 ? "spdx" : "user_owned_no_redistribution", spdx.text, "")
            }
        }
    }

    Connections {
        target: impulseResponseController
        function onImported(value): void {
            dialog.close();
        }
    }
}
