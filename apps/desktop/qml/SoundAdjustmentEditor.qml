//! Authoritative draft owner for one adjustment version. The timeline owns
//! direct manipulation; this compact inspector owns validation, precise
//! readback, revert/reset policy, and explicit publication.

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
    readonly property bool identity: trimStartMillis === 0
        && trimEndMillis === sourceDurationMillis
        && fadeInMillis === 0 && fadeOutMillis === 0 && gainCentibels === 0
    readonly property bool dirty: asset && (
        trimStartMillis !== Number(asset.trimStartMillis)
        || trimEndMillis !== Number(asset.trimEndMillis)
        || fadeInMillis !== Number(asset.fadeInMillis)
        || fadeOutMillis !== Number(asset.fadeOutMillis)
        || gainCentibels !== Number(asset.gainCentibels))

    signal saveRequested(int startMillis, int endMillis, int fadeIn, int fadeOut, int gain)

    implicitHeight: 112
    color: Theme.surfaceSubtle
    radius: Theme.controlRadius
    border.color: dirty ? Theme.accent : Theme.border

    function clamp(value: real, minimum: real, maximum: real) : int {
        return Math.round(Math.max(minimum, Math.min(maximum, value)))
    }

    function formatDuration(millis: int) : string {
        const safeMillis = Math.max(0, millis)
        const totalSeconds = Math.floor(safeMillis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        const tenths = Math.floor((safeMillis % 1000) / 100)
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds + "." + tenths
    }

    function formatGain(centibels: int) : string {
        return (centibels >= 0 ? "+" : "")
            + (centibels / 100).toFixed(1) + " dB"
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

    function setTrimRange(startMillis: int, endMillis: int) : void {
        if (!canAdjust) return
        const minimumSelection = Math.min(1000,
            Math.max(50, Math.round(sourceDurationMillis / 10000)))
        trimStartMillis = clamp(startMillis, 0,
            Math.max(0, trimEndMillis - minimumSelection))
        trimEndMillis = clamp(endMillis,
            Math.min(sourceDurationMillis, trimStartMillis + minimumSelection),
            sourceDurationMillis)
        normalizeFades()
    }

    function setFades(fadeIn: int, fadeOut: int) : void {
        if (!canAdjust) return
        fadeInMillis = clamp(fadeIn, 0, selectedDurationMillis)
        fadeOutMillis = clamp(fadeOut, 0,
            Math.max(0, selectedDurationMillis - fadeInMillis))
    }

    function setGain(centibels: int) : void {
        gainCentibels = clamp(centibels, -2400, 1200)
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
            spacing: 8

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Adjustments")
                hint: editor.dirty
                    ? qsTr("Unsaved changes")
                    : editor.asset && Number(editor.asset.adjustmentRevision) > 0
                        ? qsTr("Saved version") : qsTr("No adjustments")
            }

            EchoButton {
                text: qsTr("Revert")
                ghost: true
                enabled: editor.canAdjust && editor.dirty
                onClicked: editor.syncFromAsset()
            }

            EchoButton {
                text: qsTr("Clear")
                ghost: true
                enabled: editor.canAdjust && !editor.identity
                onClicked: editor.resetDraft()
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
            spacing: 8

            Repeater {
                model: [
                    { label: qsTr("In"), value: editor.formatDuration(editor.trimStartMillis) },
                    { label: qsTr("Out"), value: editor.formatDuration(editor.trimEndMillis) },
                    { label: qsTr("Duration"), value: editor.formatDuration(editor.selectedDurationMillis) },
                    { label: qsTr("Fade in"), value: editor.formatDuration(editor.fadeInMillis) },
                    { label: qsTr("Fade out"), value: editor.formatDuration(editor.fadeOutMillis) },
                    { label: qsTr("Gain"), value: editor.formatGain(editor.gainCentibels) }
                ]

                delegate: Rectangle {
                    required property var modelData

                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    radius: Theme.compactControlRadius
                    color: Theme.panelRaised
                    border.color: Theme.border

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 9
                        anchors.rightMargin: 9
                        spacing: 6

                        Text {
                            text: modelData.label
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            text: modelData.value
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontMeta
                            font.bold: true
                        }
                    }
                }
            }
        }
    }
}
