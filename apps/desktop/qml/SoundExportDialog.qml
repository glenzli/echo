//! Focused WAV publication surface for the sound editor. File selection and
//! job presentation live here; rendering and provenance remain native owners.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

Popup {
    id: dialog

    required property var asset
    required property var draft
    required property var exporter

    property url destination

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(500, parent.width - 40)
    padding: 0
    modal: true
    dim: true
    focus: true
    closePolicy: exporter.running ? Popup.NoAutoClose : Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function present(): void {
        destination = "";
        open();
    }

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function startExport(): void {
        if (!asset || draft.dirty || destination.toString().length === 0)
            return;
        exporter.exportAdjusted(asset.id, Number(asset.adjustmentRevision || 0), asset.path, destination, draft.trimStartMillis, draft.trimEndMillis, draft.fadeInMillis, draft.fadeOutMillis, draft.fadeInCurve, draft.fadeOutCurve, draft.gainCentibels, draft.lowCutHertz, draft.restorationValue(), draft.deHumValue(), draft.deClickValue(), draft.channelRepairValue(), draft.equalizerEnabled, draft.equalizerBands, draft.compressorEnabled, draft.compressorThresholdCentibels, draft.compressorRatioTenths, draft.compressorAttackMillis, draft.compressorReleaseMillis, draft.compressorMakeupCentibels, draft.reverbValue(), draft.limiterEnabled, draft.limiterCeilingCentibels, draft.limiterReleaseMillis, draft.effectChain, draft.editSegments, draft.effectMasks);
    }

    function debugExport(destinationUrl: url): void {
        destination = destinationUrl;
        startExport();
    }

    background: Rectangle {
        radius: Theme.controlRadius + 3
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    FileDialog {
        id: destinationDialog

        title: qsTr("Export WAV")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "wav"
        nameFilters: [qsTr("WAV audio (*.wav)")]
        onAccepted: dialog.destination = selectedFile
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
                    text: qsTr("Export adjusted sound")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: dialog.asset ? dialog.fileName(dialog.asset.path) : ""
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideMiddle
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

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 62
                radius: Theme.controlRadius
                color: Theme.surfaceSubtle
                border.width: 1
                border.color: Theme.border

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 10

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Text {
                            text: qsTr("WAV · 24-bit PCM · 48 kHz · Stereo")
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                        }

                        Text {
                            Layout.fillWidth: true
                            text: dialog.destination.toString().length > 0 ? dialog.fileName(dialog.destination.toString()) : qsTr("Choose where to save the rendered file")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideMiddle
                        }
                    }

                    EchoButton {
                        text: qsTr("Choose…")
                        ghost: true
                        enabled: !dialog.exporter.running
                        onClicked: destinationDialog.open()
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                visible: dialog.draft.dirty
                text: qsTr("Save the current adjustments before exporting so the file keeps an exact source revision.")
                color: Theme.warningText
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 7
                visible: dialog.exporter.running

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: qsTr("Rendering in the background…")
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                    }

                    Item {
                        Layout.fillWidth: true
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
            }

            Text {
                Layout.fillWidth: true
                visible: dialog.exporter.hasResult
                text: dialog.exporter.errorText.length > 0 ? qsTr("WAV created, but Echo could not save its source record.") : qsTr("WAV created · %1 LUFS · %2 dBTP").arg(dialog.exporter.integratedLufs.toFixed(1)).arg(dialog.exporter.truePeakDbtp.toFixed(1))
                color: dialog.exporter.errorText.length > 0 ? Theme.warningText : Theme.accentSelectionText
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            Text {
                Layout.fillWidth: true
                visible: !dialog.exporter.running && !dialog.exporter.hasResult && dialog.exporter.errorText.length > 0
                text: qsTr("Echo could not export this sound. Check the destination and try again.")
                color: Theme.warningText
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Item {
                    Layout.fillWidth: true
                }

                EchoButton {
                    visible: dialog.exporter.running
                    text: qsTr("Cancel")
                    ghost: true
                    onClicked: dialog.exporter.cancel()
                }

                EchoButton {
                    visible: !dialog.exporter.running
                    text: qsTr("Export WAV")
                    enabled: !dialog.draft.dirty && dialog.destination.toString().length > 0 && dialog.asset && dialog.asset.pathStatus !== "missing"
                    onClicked: dialog.startExport()
                }
            }
        }
    }
}
