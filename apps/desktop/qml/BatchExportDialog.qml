//! Batch delivery surface for the current Sound Wall projection. Native code
//! owns rendering, recovery and publication; this component only configures
//! one bounded batch and presents its state.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

Popup {
    id: dialog

    required property var assets
    required property var exporter
    required property string collectionName

    property url destination
    readonly property string selectedFormat: deliverySettings.formatKey

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(560, parent.width - 40)
    padding: 0
    modal: true
    dim: true
    focus: true
    closePolicy: exporter.running ? Popup.NoAutoClose
                                  : Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function present() : void {
        if (!exporter.recoverable && !exporter.hasResult) destination = ""
        open()
    }

    function folderName(path: string) : string {
        const normalized = path.replace(/\\/g, "/").replace(/\/$/, "")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    background: Rectangle {
        radius: Theme.controlRadius + 3
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    FolderDialog {
        id: destinationDialog
        title: qsTr("Choose export folder")
        onAccepted: dialog.destination = selectedFolder
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 18
            spacing: 10

            Rectangle {
                Layout.preferredWidth: 38
                Layout.preferredHeight: 38
                radius: 10
                color: Theme.accentSurfaceQuiet

                EchoIcon {
                    anchors.centerIn: parent
                    source: "qrc:/EchoDesktop/icons/export.svg"
                    size: 19
                    color: Theme.accentSelectionText
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("Export current results")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("%1 · %2 sounds").arg(dialog.collectionName).arg(dialog.assets.length)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideRight
                }
            }

            EchoButton {
                text: qsTr("Close")
                ghost: true
                enabled: !dialog.exporter.running
                onClicked: dialog.close()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: 20
            spacing: 14

            Text {
                Layout.fillWidth: true; wrapMode: Text.WordWrap
                font.pixelSize: Theme.fontMeta; color: Theme.textSecondary
                text: qsTr("Source labels are embedded in the audio file. Private notes and local paths stay in Echo.")
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                visible: dialog.exporter.recoverable && !dialog.exporter.running
                radius: Theme.controlRadius
                color: Theme.accentSurfaceQuiet
                border.width: 1
                border.color: Theme.accent

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 12

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        Text {
                            text: qsTr("An unfinished export can continue")
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                        }
                        Text {
                            text: qsTr("%1 of %2 finished · %3 failed")
                                .arg(dialog.exporter.completedCount)
                                .arg(dialog.exporter.totalCount)
                                .arg(dialog.exporter.failedCount)
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                        }
                    }

                    EchoButton {
                        text: qsTr("Discard")
                        ghost: true
                        onClicked: dialog.exporter.dismiss()
                    }

                    EchoButton {
                        text: qsTr("Continue")
                        onClicked: dialog.exporter.resume()
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 12
                visible: !dialog.exporter.recoverable && !dialog.exporter.running
                    && !dialog.exporter.hasResult

                Text {
                    text: qsTr("FORMAT")
                    color: Theme.textSecondary
                    font.pixelSize: 9
                    font.bold: true
                    font.letterSpacing: 0.8
                }

                AudioExportSettings { id:deliverySettings;Layout.fillWidth:true }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Uses each sound’s saved adjustments.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                Text {
                    text: qsTr("DESTINATION")
                    color: Theme.textSecondary
                    font.pixelSize: 9
                    font.bold: true
                    font.letterSpacing: 0.8
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 56
                    radius: Theme.controlRadius
                    color: Theme.surfaceSubtle
                    border.width: 1
                    border.color: Theme.border

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 10

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text {
                                text: dialog.destination.toString().length > 0
                                    ? dialog.folderName(dialog.destination.toString())
                                    : qsTr("Choose a folder")
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                elide: Text.ElideMiddle
                            }
                            Text {
                                text: qsTr("Existing files are kept; Echo adds -2, -3, and so on.")
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }
                        }

                        EchoButton {
                            text: qsTr("Choose…")
                            ghost: true
                            onClicked: destinationDialog.open()
                        }
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8
                visible: dialog.exporter.running

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        Layout.fillWidth: true
                        text: dialog.exporter.currentName.length > 0
                            ? dialog.exporter.currentName : qsTr("Preparing batch…")
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        elide: Text.ElideRight
                    }
                    Text {
                        text: Math.round(dialog.exporter.progress * 100) + "%"
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }

                ProgressBar {
                    Layout.fillWidth: true
                    from: 0
                    to: 1
                    value: dialog.exporter.progress
                }

                Text {
                    text: qsTr("%1 finished · %2 failed · %3 total")
                        .arg(dialog.exporter.completedCount)
                        .arg(dialog.exporter.failedCount)
                        .arg(dialog.exporter.totalCount)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 6
                visible: dialog.exporter.hasResult

                Text {
                    text: dialog.exporter.failedCount === 0
                        ? qsTr("Export complete") : qsTr("Export complete with some failures")
                    color: dialog.exporter.failedCount === 0
                        ? Theme.accentSelectionText : Theme.warningText
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                }
                Text {
                    text: qsTr("%1 files created in %2 · %3 failed")
                        .arg(dialog.exporter.completedCount)
                        .arg(dialog.folderName(dialog.exporter.outputDirectory))
                        .arg(dialog.exporter.failedCount)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontBody
                }
            }

            Text {
                Layout.fillWidth: true
                visible: dialog.exporter.errorText.length > 0
                text: qsTr("Echo could not start this export. Check the destination and try again.")
                color: Theme.warningText
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }

                EchoButton {
                    visible: dialog.exporter.running
                    text: qsTr("Cancel remaining")
                    ghost: true
                    onClicked: dialog.exporter.cancel()
                }

                EchoButton {
                    visible: dialog.exporter.hasResult
                    text: qsTr("Done")
                    onClicked: {
                        dialog.exporter.dismiss()
                        dialog.close()
                    }
                }

                EchoButton {
                    visible: !dialog.exporter.running && !dialog.exporter.recoverable
                        && !dialog.exporter.hasResult
                    text: qsTr("Export %1 sounds").arg(dialog.assets.length)
                    enabled: dialog.assets.length > 0
                        && dialog.destination.toString().length > 0
                    onClicked: { dialog.exporter.exportOptions=deliverySettings.options; dialog.exporter.start(
                        dialog.assets, dialog.destination, dialog.selectedFormat); }
                }
            }
        }
    }
}
