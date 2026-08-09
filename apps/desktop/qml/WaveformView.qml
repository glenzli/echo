//! WaveformView renders a cached min/max pyramid as a continuous antialiased
//! envelope. Geometry is interpolated for presentation; evidence remains the
//! immutable cached buckets supplied by the audio engine.

import QtQuick

Canvas {
    id: root

    property var levels: []
    property color fillColor: Theme.accent
    property color progressColor: Qt.lighter(Theme.accent, 1.35)
    property real progress: 0.0
    property bool paintReady: false

    readonly property int selectedLevel: pickLevel()

    antialiasing: true
    opacity: paintReady ? 1 : 0
    renderStrategy: Canvas.Immediate

    function pickLevel() : int {
        if (levels.length === 0) {
            return -1
        }
        // Prefer enough source buckets for roughly one envelope point per
        // device pixel. If none reaches that density, keep the finest level
        // and interpolate it visually instead of drawing oversized bars.
        for (let index = levels.length - 1; index >= 0; --index) {
            if (levels[index].mins.length >= width) {
                return index
            }
        }
        let finest = 0
        for (let index = 1; index < levels.length; ++index) {
            if (levels[index].mins.length > levels[finest].mins.length) {
                finest = index
            }
        }
        return finest
    }

    function traceSmoothed(context: var, values: var, centerY: real,
                           scale: real, reverse: bool) : void {
        const count = values.length
        if (count === 1) {
            const y = centerY + values[0] * scale
            context.lineTo(width, y)
            return
        }
        const start = reverse ? count - 1 : 0
        const end = reverse ? 0 : count - 1
        const direction = reverse ? -1 : 1
        let previousX = start / (count - 1) * width
        let previousY = centerY + values[start] * scale
        context.lineTo(previousX, previousY)
        for (let index = start + direction; reverse ? index >= end : index <= end;
             index += direction) {
            const x = index / (count - 1) * width
            const y = centerY + values[index] * scale
            const midpointX = (previousX + x) / 2
            const midpointY = (previousY + y) / 2
            context.quadraticCurveTo(previousX, previousY, midpointX, midpointY)
            previousX = x
            previousY = y
        }
        context.lineTo(previousX, previousY)
    }

    function drawEnvelope(context: var, mins: var, maxs: var,
                          centerY: real, scale: real, color: color) : void {
        context.beginPath()
        traceSmoothed(context, mins, centerY, scale, false)
        traceSmoothed(context, maxs, centerY, scale, true)
        context.closePath()
        context.fillStyle = color
        context.fill()
    }

    onLevelsChanged: {
        paintReady = false
        requestPaint()
    }
    onWidthChanged: {
        paintReady = false
        requestPaint()
    }
    onHeightChanged: {
        paintReady = false
        requestPaint()
    }
    onProgressChanged: requestPaint()
    onFillColorChanged: requestPaint()
    onProgressColorChanged: requestPaint()

    onPaint: {
        const context = getContext("2d")
        context.reset()
        context.clearRect(0, 0, width, height)
        if (selectedLevel < 0) {
            paintReady = true
            return
        }
        const level = levels[selectedLevel]
        const mins = level.mins
        const maxs = level.maxs
        const bucketCount = mins.length
        if (bucketCount === 0) {
            paintReady = true
            return
        }

        let peak = 0.0
        for (let index = 0; index < bucketCount; ++index) {
            peak = Math.max(peak, Math.abs(mins[index]), Math.abs(maxs[index]))
        }
        if (peak <= 0.0) {
            peak = 1.0
        }

        const centerY = height / 2
        const scale = (height / 2 - 3) / peak

        context.strokeStyle = Theme.borderStrong
        context.lineWidth = 1
        context.globalAlpha = 0.55
        context.beginPath()
        context.moveTo(0, centerY + 0.5)
        context.lineTo(width, centerY + 0.5)
        context.stroke()
        context.globalAlpha = 0.88
        drawEnvelope(context, mins, maxs, centerY, scale, fillColor)

        const playedWidth = Math.max(0, Math.min(width, progress * width))
        if (playedWidth > 0) {
            context.save()
            context.beginPath()
            context.rect(0, 0, playedWidth, height)
            context.clip()
            context.globalAlpha = 1.0
            drawEnvelope(context, mins, maxs, centerY, scale, progressColor)
            context.restore()
        }
        context.globalAlpha = 1.0
        paintReady = true
    }
}
