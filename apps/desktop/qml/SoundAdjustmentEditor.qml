//! Draft owner for the first non-destructive sound-processing slice.
//! Milliseconds and centibels mirror the persisted AdjustmentGraph exactly;
//! preview and save are explicit so browsing never mutates an original.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: editor

    required property var asset

    property int trimStartMillis: 0
    property int trimEndMillis: 0
    property int fadeInMillis: 0
    property int fadeOutMillis: 0
    property int gainCentibels: 0

    readonly property int sourceDurationMillis: asset ? Number(asset.durationMillis) : 0
    readonly property int selectedDurationMillis: Math.max(0, trimEndMillis - trimStartMillis)
    readonly property bool canAdjust: sourceDurationMillis > 0
    readonly property bool dirty: asset && (
        trimStartMillis !== Number(asset.trimStartMillis)
        || trimEndMillis !== Number(asset.trimEndMillis)
        || fadeInMillis !== Number(asset.fadeInMillis)
        || fadeOutMillis !== Number(asset.fadeOutMillis)
        || gainCentibels !== Number(asset.gainCentibels))

    signal previewRequested(int startMillis, int endMillis, int fadeIn, int fadeOut, int gain)
    signal saveRequested(int startMillis, int endMillis, int fadeIn, int fadeOut, int gain)

    implicitHeight: 202
    color: Theme.surfaceSubtle
    radius: Theme.controlRadius
    border.color: dirty ? Theme.accent : Theme.border

    function formatDuration(millis: int) : string {
        const safeMillis = Math.max(0, millis)
        const totalSeconds = Math.floor(safeMillis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        const tenths = Math.floor((safeMillis % 1000) / 100)
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds + "." + tenths
    }

    function syncFromAsset() : void {
        if (!asset) {
            trimStartMillis = 0
            trimEndMillis = 0
            fadeInMillis = 0
            fadeOutMillis = 0
            gainCentibels = 0
            return
        }
        trimStartMillis = Number(asset.trimStartMillis)
        trimEndMillis = Number(asset.trimEndMillis) > 0
            ? Number(asset.trimEndMillis) : sourceDurationMillis
        fadeInMillis = Number(asset.fadeInMillis)
        fadeOutMillis = Number(asset.fadeOutMillis)
        gainCentibels = Number(asset.gainCentibels)
    }

    function resetDraft() : void {
        trimStartMillis = 0
        trimEndMillis = sourceDurationMillis
        fadeInMillis = 0
        fadeOutMillis = 0
        gainCentibels = 0
    }

    function normalizeFades() : void {
        fadeInMillis = Math.min(fadeInMillis, selectedDurationMillis)
        fadeOutMillis = Math.min(fadeOutMillis,
                                 Math.max(0, selectedDurationMillis - fadeInMillis))
    }

    onAssetChanged: Qt.callLater(syncFromAsset)
    Component.onCompleted: syncFromAsset()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        RowLayout {
            Layout.fillWidth: true

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Adjustments")
                hint: editor.dirty
                    ? qsTr("Previewing an unsaved version")
                    : editor.asset && Number(editor.asset.adjustmentRevision) > 0
                        ? qsTr("Saved version")
                        : qsTr("Original remains unchanged")
            }

            EchoButton {
                text: qsTr("Reset")
                ghost: true
                enabled: editor.canAdjust && (editor.dirty
                    || editor.trimStartMillis !== 0
                    || editor.trimEndMillis !== editor.sourceDurationMillis
                    || editor.fadeInMillis !== 0 || editor.fadeOutMillis !== 0
                    || editor.gainCentibels !== 0)
                onClicked: editor.resetDraft()
            }

            EchoButton {
                text: qsTr("Preview")
                ghost: true
                enabled: editor.canAdjust
                onClicked: editor.previewRequested(
                    editor.trimStartMillis, editor.trimEndMillis,
                    editor.fadeInMillis, editor.fadeOutMillis, editor.gainCentibels)
            }

            EchoButton {
                text: qsTr("Save version")
                enabled: editor.canAdjust && editor.dirty
                onClicked: editor.saveRequested(
                    editor.trimStartMillis, editor.trimEndMillis,
                    editor.fadeInMillis, editor.fadeOutMillis, editor.gainCentibels)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Text {
                text: qsTr("Range")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 52
            }

            Text {
                text: editor.formatDuration(editor.trimStartMillis)
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 44
            }

            RangeSlider {
                id: trimSlider
                Layout.fillWidth: true
                from: 0
                to: Math.max(1, editor.sourceDurationMillis)
                stepSize: 50
                first.value: editor.trimStartMillis
                second.value: Math.max(editor.trimStartMillis + 1, editor.trimEndMillis)
                enabled: editor.canAdjust
                first.onMoved: {
                    editor.trimStartMillis = Math.round(first.value)
                    editor.normalizeFades()
                }
                second.onMoved: {
                    editor.trimEndMillis = Math.round(second.value)
                    editor.normalizeFades()
                }
            }

            Text {
                text: editor.formatDuration(editor.trimEndMillis)
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 44
                horizontalAlignment: Text.AlignRight
            }

            Text {
                text: qsTr("Selected %1").arg(editor.formatDuration(editor.selectedDurationMillis))
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 92
                horizontalAlignment: Text.AlignRight
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 18

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: qsTr("Fade in")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 52
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: Math.max(1, Math.min(5000,
                        editor.selectedDurationMillis - editor.fadeOutMillis))
                    stepSize: 50
                    value: editor.fadeInMillis
                    enabled: editor.canAdjust
                    onMoved: editor.fadeInMillis = Math.round(value)
                }
                Text {
                    text: editor.formatDuration(editor.fadeInMillis)
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 38
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: qsTr("Fade out")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 54
                }
                Slider {
                    Layout.fillWidth: true
                    from: 0
                    to: Math.max(1, Math.min(5000,
                        editor.selectedDurationMillis - editor.fadeInMillis))
                    stepSize: 50
                    value: editor.fadeOutMillis
                    enabled: editor.canAdjust
                    onMoved: editor.fadeOutMillis = Math.round(value)
                }
                Text {
                    text: editor.formatDuration(editor.fadeOutMillis)
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 38
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: qsTr("Gain")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 38
                }
                Slider {
                    Layout.fillWidth: true
                    from: -2400
                    to: 1200
                    stepSize: 50
                    value: editor.gainCentibels
                    enabled: editor.canAdjust
                    onMoved: editor.gainCentibels = Math.round(value)
                }
                Text {
                    text: (editor.gainCentibels >= 0 ? "+" : "")
                        + (editor.gainCentibels / 100).toFixed(1) + " dB"
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 48
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }
}
