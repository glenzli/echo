//! WaveformView: renders one level of a cached waveform pyramid. The level
//! is picked automatically so the bucket count fits the current width.

import QtQuick

Canvas {
    id: root

    property var levels: []
    property color fillColor: Theme.accent
    property color progressColor: Qt.lighter(Theme.accent, 1.35)
    property real progress: 0.0

    readonly property int selectedLevel: pickLevel()

    function pickLevel() : int {
        if (levels.length === 0) {
            return -1
        }
        // Prefer the finest level whose bucket count fits the width, else
        // fall back to the coarsest.
        for (let index = 0; index < levels.length; ++index) {
            const buckets = levels[index].mins.length
            if (buckets <= width * 2) {
                return index
            }
        }
        return levels.length - 1
    }

    onLevelsChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onProgressChanged: requestPaint()

    onPaint: {
        const context = getContext("2d")
        context.clearRect(0, 0, width, height)
        if (selectedLevel < 0) {
            return
        }
        const level = levels[selectedLevel]
        const mins = level.mins
        const maxs = level.maxs
        const bucketCount = mins.length
        if (bucketCount === 0) {
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
        const scale = (height / 2 - 2) / peak
        const step = width / bucketCount
        const playedBuckets = Math.floor(progress * bucketCount)

        context.beginPath()
        for (let index = 0; index < bucketCount; ++index) {
            const x = index * step
            const top = centerY + mins[index] * scale
            const bottom = centerY + maxs[index] * scale
            const barWidth = Math.max(1.0, step - 0.5)
            if (index < playedBuckets) {
                context.fillStyle = progressColor
            } else {
                context.fillStyle = fillColor
            }
            context.fillRect(x, top, barWidth, Math.max(1.0, bottom - top))
        }
    }
}
