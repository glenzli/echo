//! Display-only, source-derived spectrogram overview. Its caller owns the
//! synchronized timeline viewport and any future repair gestures.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: spectrogram

    required property var artifact
    required property int sourceDurationMillis
    required property real viewStartRatio
    required property real viewEndRatio
    required property real progress
    required property bool hasTimeSelection
    required property real selectionStartRatio
    required property real selectionEndRatio
    required property var regions

    readonly property int timeColumns: Number(artifact.timeColumns || 0)
    readonly property int frequencyBins: Number(artifact.frequencyBins || 0)
    readonly property var magnitudes: artifact.magnitudes || []
    readonly property bool hasOverview: timeColumns > 0 && frequencyBins > 0
        && magnitudes.length === timeColumns * frequencyBins

    signal regionRequested(int startMillis, int endMillis, int lowHertz, int highHertz)
    signal clearRequested

    implicitHeight: 230
    color: Theme.panelRaised
    radius: Theme.panelRadius
    border.color: Theme.border
    clip: true

    function clamp(value: real, minimum: real, maximum: real): real {
        return Math.max(minimum, Math.min(maximum, value));
    }

    function colorForMagnitude(value: int): var {
        const normalized = clamp(value / 255, 0, 1);
        const red = Math.round(14 + 241 * Math.pow(normalized, 1.55));
        const green = Math.round(10 + 218 * Math.pow(Math.max(0, normalized - 0.30) / 0.70, 1.2));
        const blue = Math.round(30 + 194 * Math.pow(Math.max(0, normalized - 0.08) / 0.92, 0.62));
        return [red, green, blue];
    }

    onArtifactChanged: spectrumCanvas.requestPaint()
    onViewStartRatioChanged: spectrumCanvas.requestPaint()
    onViewEndRatioChanged: spectrumCanvas.requestPaint()
    onWidthChanged: spectrumCanvas.requestPaint()
    onHeightChanged: spectrumCanvas.requestPaint()

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
                text: qsTr("Original · read-only overview")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Item { Layout.fillWidth: true }

            Text {
                text: hasOverview ? qsTr("%1 repairs").arg(regions.length) : qsTr("Loading…")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
            }

            Button {
                text: qsTr("Clear repairs")
                enabled: regions.length > 0
                onClicked: spectrogram.clearRequested()
            }
        }

        Rectangle {
            id: spectrumSurface

            Layout.fillWidth: true
            Layout.fillHeight: true
            color: Theme.waveformSurface
            border.color: Theme.border
            clip: true

            Canvas {
                id: spectrumCanvas

                anchors.fill: parent
                renderTarget: Canvas.Image
                renderStrategy: Canvas.Cooperative

                onPaint: {
                    const context = getContext("2d");
                    context.reset();
                    context.clearRect(0, 0, width, height);
                    if (!spectrogram.hasOverview || width <= 0 || height <= 0)
                        return;

                    const visibleStart = Math.floor(spectrogram.clamp(spectrogram.viewStartRatio, 0, 1) * spectrogram.timeColumns);
                    const visibleEnd = Math.max(visibleStart + 1, Math.ceil(spectrogram.clamp(spectrogram.viewEndRatio, 0, 1) * spectrogram.timeColumns));
                    const sourceColumns = Math.max(1, visibleEnd - visibleStart);
                    const outputColumns = Math.max(1, Math.round(width));
                    const outputRows = Math.max(1, Math.round(height));
                    const image = context.createImageData(outputColumns, outputRows);
                    for (let y = 0; y < outputRows; ++y) {
                        const frequencyStart = Math.floor((outputRows - y - 1) * spectrogram.frequencyBins / outputRows);
                        const frequencyEnd = Math.max(frequencyStart + 1, Math.ceil((outputRows - y) * spectrogram.frequencyBins / outputRows));
                        for (let x = 0; x < outputColumns; ++x) {
                            const timeStart = visibleStart + Math.floor(x * sourceColumns / outputColumns);
                            const timeEnd = Math.min(visibleEnd, Math.max(timeStart + 1, visibleStart + Math.ceil((x + 1) * sourceColumns / outputColumns)));
                            let magnitude = 0;
                            for (let column = timeStart; column < timeEnd; ++column) {
                                for (let bin = frequencyStart; bin < frequencyEnd; ++bin)
                                    magnitude = Math.max(magnitude, Number(spectrogram.magnitudes[column * spectrogram.frequencyBins + bin] || 0));
                            }
                            const color = spectrogram.colorForMagnitude(magnitude);
                            const offset = (y * outputColumns + x) * 4;
                            image.data[offset] = color[0];
                            image.data[offset + 1] = color[1];
                            image.data[offset + 2] = color[2];
                            image.data[offset + 3] = 255;
                        }
                    }
                    context.putImageData(image, 0, 0);
                }
            }

            Text {
                anchors.centerIn: parent
                visible: !spectrogram.hasOverview
                text: qsTr("Spectrogram overview unavailable")
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
                    visible: width > 0 && x < parent.width
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
                    spectrogram.regionRequested(
                        Math.round(spectrogram.clamp(startRatio, 0, 1) * spectrogram.sourceDurationMillis),
                        Math.round(spectrogram.clamp(endRatio, 0, 1) * spectrogram.sourceDurationMillis),
                        Math.max(20, Math.min(23999, low)),
                        Math.max(21, Math.min(24000, high))
                    );
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
