//! Read-only durable processing application history with an explicit guarded
//! revert intent. The caller owns refresh, mutation and notices; this dialog
//! owns only bounded presentation and confirmation state.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: dialog

    // Entries accept the strict DesktopBackend history projection. User-authored
    // recipeName is displayed byte-for-byte and never translated.
    property var historyModel: []
    property int modelRevision: 0
    property string pendingBatchId: ""
    property string pendingRecipeName: ""

    signal revertRequested(string batchId)

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(840, parent.width - 40)
    height: Math.min(660, parent.height - 40)
    padding: 0
    modal: true
    dim: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function modelCount(): int {
        const observedRevision = dialog.modelRevision;
        if (!dialog.historyModel)
            return 0;
        if (dialog.historyModel.count !== undefined)
            return dialog.historyModel.count;
        return dialog.historyModel.length !== undefined ? dialog.historyModel.length : 0;
    }

    function formatDate(millis: double): string {
        if (!Number.isFinite(millis) || millis <= 0)
            return qsTr("Unknown time");
        return new Date(millis).toLocaleString(Qt.locale(), Locale.ShortFormat);
    }

    function mergeModeLabel(mode: string): string {
        return mode === "replace" ? qsTr("Replaced current processing") : qsTr("Merged selected processing");
    }

    function applicationSummary(entry: var): string {
        return qsTr("%1 updated · %2 unchanged · %3 failed").arg(Number(entry.updatedCount || 0)).arg(Number(entry.unchangedCount || 0)).arg(Number(entry.failedCount || 0));
    }

    function revertSummary(entry: var): string {
        return qsTr("%1 restored · %2 unchanged · %3 conflicts · %4 failed").arg(Number(entry.restoredCount || 0)).arg(Number(entry.revertUnchangedCount || 0)).arg(Number(entry.conflictCount || 0)).arg(Number(entry.revertFailedCount || 0));
    }

    function beginRevert(entry: var): void {
        if (!entry || Boolean(entry.reverted))
            return;
        pendingBatchId = String(entry.batchId || "");
        pendingRecipeName = String(entry.recipeName || "");
        if (pendingBatchId.length > 0)
            revertConfirmation.open();
    }

    function present(): void {
        pendingBatchId = "";
        pendingRecipeName = "";
        open();
    }

    background: Rectangle {
        radius: Theme.panelRadius
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 18
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("Processing history")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Recipe applications and their durable undo results")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            EchoButton {
                text: qsTr("Close")
                ghost: true
                onClicked: dialog.close()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: historyList

                anchors.fill: parent
                anchors.margins: 12
                model: dialog.historyModel
                spacing: 8
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {
                    policy: ScrollBar.AsNeeded
                }

                delegate: Rectangle {
                    id: historyRow

                    required property var modelData

                    width: ListView.view.width
                    implicitHeight: rowContent.implicitHeight + 24
                    radius: Theme.controlRadius
                    color: Theme.panel
                    border.width: 1
                    border.color: Theme.border

                    ColumnLayout {
                        id: rowContent

                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 12
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                Layout.fillWidth: true
                                text: String(historyRow.modelData.recipeName || "")
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Rectangle {
                                implicitWidth: statusLabel.implicitWidth + 14
                                implicitHeight: 24
                                radius: 12
                                color: Boolean(historyRow.modelData.reverted) ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle

                                Text {
                                    id: statusLabel
                                    anchors.centerIn: parent
                                    text: Boolean(historyRow.modelData.reverted) ? qsTr("Undone") : qsTr("Undo available")
                                    color: Boolean(historyRow.modelData.reverted) ? Theme.accentSelectionText : Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    font.weight: Font.DemiBold
                                }
                            }

                            Text {
                                text: dialog.formatDate(Number(historyRow.modelData.createdAtMillis || 0))
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                text: qsTr("Version %1").arg(Number(historyRow.modelData.recipeRevisionNumber || 1))
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }

                            Rectangle {
                                Layout.preferredWidth: 1
                                Layout.preferredHeight: 12
                                color: Theme.border
                            }

                            Text {
                                text: dialog.mergeModeLabel(String(historyRow.modelData.mergeMode || "merge"))
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }

                            Rectangle {
                                Layout.preferredWidth: 1
                                Layout.preferredHeight: 12
                                color: Theme.border
                            }

                            Text {
                                text: qsTr("%1 sounds").arg(Number(historyRow.modelData.targetCount || 0))
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }

                            Item {
                                Layout.fillWidth: true
                            }

                            EchoButton {
                                visible: !Boolean(historyRow.modelData.reverted)
                                text: qsTr("Undo batch")
                                ghost: true
                                onClicked: dialog.beginRevert(historyRow.modelData)
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 1
                            color: Theme.border
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Text {
                                Layout.fillWidth: true
                                text: dialog.applicationSummary(historyRow.modelData)
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                                elide: Text.ElideRight
                            }

                            Text {
                                visible: Boolean(historyRow.modelData.reverted)
                                text: dialog.revertSummary(historyRow.modelData)
                                color: Number(historyRow.modelData.conflictCount || 0) > 0 || Number(historyRow.modelData.revertFailedCount || 0) > 0 ? Theme.warningText : Theme.accentSelectionText
                                font.pixelSize: Theme.fontMeta
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(420, parent.width - 40)
                visible: dialog.modelCount() === 0
                spacing: 6

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("No processing history")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Applied processing recipes will appear here.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Text {
            Layout.fillWidth: true
            Layout.margins: 12
            text: qsTr("This view shows the 100 most recent application batches.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            horizontalAlignment: Text.AlignHCenter
        }
    }

    Popup {
        id: revertConfirmation

        parent: Overlay.overlay
        x: Math.round((parent.width - width) / 2)
        y: Math.round((parent.height - height) / 2)
        width: Math.min(460, parent.width - 40)
        padding: 18
        modal: true
        dim: true
        focus: true
        closePolicy: Popup.CloseOnEscape

        background: Rectangle {
            radius: Theme.panelRadius
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 12

            Text {
                Layout.fillWidth: true
                text: qsTr("Undo processing batch?")
                color: Theme.textPrimary
                font.pixelSize: 15
                font.weight: Font.DemiBold
            }

            Text {
                Layout.fillWidth: true
                text: dialog.pendingRecipeName.length > 0 ? dialog.pendingRecipeName : qsTr("Processing recipe")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Only sounds untouched since this batch will be restored. Later edits remain unchanged and are reported as conflicts.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Item {
                    Layout.fillWidth: true
                }

                EchoButton {
                    text: qsTr("Cancel")
                    ghost: true
                    onClicked: revertConfirmation.close()
                }

                EchoButton {
                    text: qsTr("Undo batch")
                    onClicked: {
                        const batchId = dialog.pendingBatchId;
                        revertConfirmation.close();
                        if (batchId.length > 0)
                            dialog.revertRequested(batchId);
                    }
                }
            }
        }
    }
}
