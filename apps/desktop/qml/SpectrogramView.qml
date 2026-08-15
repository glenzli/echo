//! Display-only, source-derived spectrogram overview. Its caller owns the
//! synchronized timeline viewport and any future repair gestures.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: spectrogram

    required property string imageUrl
    required property bool loading
    required property int sourceDurationMillis
    required property real viewStartRatio
    required property real viewEndRatio
    required property real progress
    required property bool hasTimeSelection
    required property real selectionStartRatio
    required property real selectionEndRatio
    required property bool layerEnabled
    required property var regions
    required property bool canCreateRenderedWorkingCopy
    required property bool renderedWorkingCopyRunning
    required property bool renderedWorkingCopyReady
    required property string renderedWorkingCopyError
    required property bool renderedEraseMode
    required property int renderedWorkingCopyOperationCount

    readonly property bool hasOverview: imageUrl.length > 0

    signal regionRequested(int startMillis, int endMillis, int lowHertz, int highHertz)
    signal layerEnabledRequested(bool enabled)
    signal clearRequested
    signal renderedWorkingCopyRequested
    signal renderedEraseModeRequested(bool enabled)
    signal renderedEraseRequested(int startMillis, int endMillis, int lowHertz, int highHertz)
    signal renderedWorkingCopyAuditionRequested

    implicitHeight: 230
    color: Theme.panelRaised
    radius: Theme.panelRadius
    border.color: Theme.border
    clip: true

    function clamp(value: real, minimum: real, maximum: real): real {
        return Math.max(minimum, Math.min(maximum, value));
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 6

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: qsTr("Spectrogram")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.bold: true
            }

            Text {
                text: spectrogram.renderedEraseMode
                    ? qsTr("Rendered working layer · destructive")
                    : qsTr("Original-first · non-destructive")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Switch {
                text: qsTr("Spectral adjustment")
                checked: spectrogram.layerEnabled
                onClicked: spectrogram.layerEnabledRequested(checked)
            }

            Item { Layout.fillWidth: true }

            Text {
                text: hasOverview ? (layerEnabled ? qsTr("%1 repairs").arg(regions.length) : qsTr("Bypassed"))
                    : (loading ? qsTr("Loading…") : qsTr("Unavailable"))
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }

            Button {
                text: qsTr("Clear repairs")
                enabled: regions.length > 0
                onClicked: spectrogram.clearRequested()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: qsTr("Post-effect repair")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Button {
                text: spectrogram.renderedWorkingCopyRunning ? qsTr("Freezing render…")
                    : (spectrogram.renderedWorkingCopyReady ? qsTr("Rendered repair ready") : qsTr("Create working copy"))
                enabled: spectrogram.canCreateRenderedWorkingCopy && !spectrogram.renderedWorkingCopyRunning
                    && !spectrogram.renderedWorkingCopyReady
                onClicked: spectrogram.renderedWorkingCopyRequested()
            }

            Switch {
                visible: spectrogram.renderedWorkingCopyReady
                text: qsTr("Erase mode")
                checked: spectrogram.renderedEraseMode
                enabled: !spectrogram.renderedWorkingCopyRunning
                onToggled: spectrogram.renderedEraseModeRequested(checked)
            }

            Button {
                visible: spectrogram.renderedWorkingCopyReady
                text: qsTr("Audition rendered")
                enabled: !spectrogram.renderedWorkingCopyRunning
                onClicked: spectrogram.renderedWorkingCopyAuditionRequested()
            }

            Text {
                Layout.fillWidth: true
                text: spectrogram.renderedWorkingCopyError.length > 0 ? spectrogram.renderedWorkingCopyError
                    : (spectrogram.renderedWorkingCopyReady
                        ? (spectrogram.renderedEraseMode
                            ? qsTr("Draw a region to erase it in the rendered working layer.")
                            : qsTr("%1 committed repairs. Upstream changes require a new copy.").arg(spectrogram.renderedWorkingCopyOperationCount))
                        : qsTr("Save adjustments before creating a post-effect repair copy."))
                color: spectrogram.renderedWorkingCopyError.length > 0 ? Theme.warningText : Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        Rectangle {
            id: spectrumSurface

            Layout.fillWidth: true
            Layout.fillHeight: true
            color: Theme.waveformSurface
            border.color: Theme.border
            clip: true

            Image {
                id: spectrumImage
                anchors.fill: parent
                source: spectrogram.imageUrl
                fillMode: Image.Stretch
                smooth: true
                sourceClipRect: Qt.rect(
                    Math.floor(spectrogram.clamp(spectrogram.viewStartRatio, 0, 1) * sourceSize.width),
                    0,
                    Math.max(1, Math.ceil((spectrogram.clamp(spectrogram.viewEndRatio, 0, 1) - spectrogram.clamp(spectrogram.viewStartRatio, 0, 1)) * sourceSize.width)),
                    sourceSize.height
                )
            }

            Text {
                anchors.centerIn: parent
                visible: !spectrogram.hasOverview
                text: spectrogram.loading ? qsTr("Loading…") : qsTr("Spectrogram overview unavailable")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }

            Rectangle {
                x: spectrogram.clamp((spectrogram.selectionStartRatio - spectrogram.viewStartRatio) / Math.max(0.0001, spectrogram.viewEndRatio - spectrogram.viewStartRatio), 0, 1) * parent.width
                width: Math.max(0, spectrogram.clamp((spectrogram.selectionEndRatio - spectrogram.viewStartRatio) / Math.max(0.0001, spectrogram.viewEndRatio - spectrogram.viewStartRatio), 0, 1) * parent.width - x)
                height: parent.height
                color: Theme.accentSurface
                border.color: Theme.accent
                opacity: 0.34
                visible: spectrogram.hasTimeSelection && width > 0
            }

            Repeater {
                model: spectrogram.regions

                delegate: Rectangle {
                    required property var modelData

                    readonly property real visibleDuration: Math.max(0.0001, spectrogram.viewEndRatio - spectrogram.viewStartRatio)
                    x: Math.max(0, (Number(modelData.startMillis) / Math.max(1, spectrogram.sourceDurationMillis) - spectrogram.viewStartRatio) / visibleDuration * parent.width)
                    width: Math.max(0, (Number(modelData.endMillis) / Math.max(1, spectrogram.sourceDurationMillis) - spectrogram.viewStartRatio) / visibleDuration * parent.width - x)
                    y: Math.max(0, (1 - Number(modelData.highHertz) / 24000) * parent.height)
                    height: Math.max(0, (Number(modelData.highHertz) - Number(modelData.lowHertz)) / 24000 * parent.height)
                    color: Theme.accentSurface
                    border.color: Theme.accent
                    opacity: 0.56
                    visible: spectrogram.layerEnabled && width > 0 && x < parent.width
                }
            }

            Rectangle {
                id: pendingRegion

                property real startX: 0
                property real startY: 0
                property real currentX: 0
                property real currentY: 0
                readonly property real leftEdge: Math.min(startX, currentX)
                readonly property real topEdge: Math.min(startY, currentY)

                x: leftEdge
                y: topEdge
                width: Math.abs(currentX - startX)
                height: Math.abs(currentY - startY)
                visible: spectrumInput.pressed && width >= 4 && height >= 4
                color: Theme.accentSurface
                border.color: Theme.accent
                opacity: 0.7
            }

            MouseArea {
                id: spectrumInput

                anchors.fill: parent
                acceptedButtons: Qt.LeftButton
                enabled: spectrogram.layerEnabled || spectrogram.renderedEraseMode
                cursorShape: Qt.CrossCursor
                onPressed: function(mouse) {
                    pendingRegion.startX = mouse.x;
                    pendingRegion.startY = mouse.y;
                    pendingRegion.currentX = mouse.x;
                    pendingRegion.currentY = mouse.y;
                }
                onPositionChanged: function(mouse) {
                    if (pressed) {
                        pendingRegion.currentX = mouse.x;
                        pendingRegion.currentY = mouse.y;
                    }
                }
                onReleased: function(mouse) {
                    pendingRegion.currentX = mouse.x;
                    pendingRegion.currentY = mouse.y;
                    if (pendingRegion.width < 8 || pendingRegion.height < 8 || spectrogram.sourceDurationMillis <= 0)
                        return;
                    const viewDuration = Math.max(0.0001, spectrogram.viewEndRatio - spectrogram.viewStartRatio);
                    const startRatio = spectrogram.viewStartRatio + pendingRegion.leftEdge / width * viewDuration;
                    const endRatio = spectrogram.viewStartRatio + (pendingRegion.leftEdge + pendingRegion.width) / width * viewDuration;
                    const high = Math.round((1 - pendingRegion.topEdge / height) * 24000);
                    const low = Math.round((1 - (pendingRegion.topEdge + pendingRegion.height) / height) * 24000);
                    const startMillis = Math.round(spectrogram.clamp(startRatio, 0, 1) * spectrogram.sourceDurationMillis);
                    const endMillis = Math.round(spectrogram.clamp(endRatio, 0, 1) * spectrogram.sourceDurationMillis);
                    const lowHertz = Math.max(20, Math.min(23999, low));
                    const highHertz = Math.max(21, Math.min(24000, high));
                    if (spectrogram.renderedEraseMode) {
                        spectrogram.renderedEraseRequested(startMillis, endMillis, lowHertz, highHertz);
                    } else {
                        spectrogram.regionRequested(startMillis, endMillis, lowHertz, highHertz);
                    }
                }
            }

            Rectangle {
                x: spectrogram.clamp((spectrogram.progress - spectrogram.viewStartRatio) / Math.max(0.0001, spectrogram.viewEndRatio - spectrogram.viewStartRatio), 0, 1) * parent.width
                width: 1
                height: parent.height
                color: Theme.textPrimary
                visible: spectrogram.progress >= spectrogram.viewStartRatio && spectrogram.progress <= spectrogram.viewEndRatio
            }

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 7
                anchors.top: parent.top
                anchors.topMargin: 5
                text: "24 kHz"
                color: Theme.textDisabled
                font.pixelSize: 9
            }

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 7
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 5
                text: "0 Hz"
                color: Theme.textDisabled
                font.pixelSize: 9
            }
        }
    }
}
