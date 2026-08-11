//! WaveformView renders a cached min/max pyramid as either a continuous
//! envelope or compact rounded bars. Geometry is interpolated or aggregated
//! only for presentation; evidence remains the immutable cached buckets
//! supplied by the audio engine.

import QtQuick

Canvas {
    id: root

    property var levels: []
    property color fillColor: Theme.waveformFill
    property color progressColor: Theme.waveformPlayed
    property color centerLineColor: Theme.waveformCenter
    property color outlineColor: Theme.effectiveDark ? "#c3c9cf" : "#4f5963"
    property string renderMode: "envelope"
    property bool normalize: true
    property real normalizationFloor: 0.32
    property real peakHeadroom: 1.08
    property real amplitudeExponent: 0.78
    property real verticalPadding: 4
    property real barWidth: 2.25
    property real barGap: 1.5
    property real barRadius: 1.1
    property real progress: 0.0
    property real viewStartRatio: 0.0
    property real viewEndRatio: 1.0
    property bool paintReady: false

    readonly property int selectedLevel: pickLevel()

    antialiasing: true
    opacity: paintReady ? 1 : 0
    renderStrategy: Canvas.Immediate

    function pickLevel(): int {
        if (levels.length === 0) {
            return -1;
        }
        const visibleRatio = Math.max(0.000001, Math.min(1, viewEndRatio) - Math.max(0, viewStartRatio));
        // Prefer enough source buckets for roughly one envelope point per
        // device pixel. If none reaches that density, keep the finest level
        // and interpolate it visually instead of drawing oversized bars.
        for (let index = levels.length - 1; index >= 0; --index) {
            if (levels[index].mins.length * visibleRatio >= width) {
                return index;
            }
        }
        let finest = 0;
        for (let index = 1; index < levels.length; ++index) {
            if (levels[index].mins.length > levels[finest].mins.length) {
                finest = index;
            }
        }
        return finest;
    }

    function displayY(value: real, peak: real, centerY: real, scale: real): real {
        const normalized = Math.min(1, Math.abs(value) / peak);
        const shaped = Math.pow(normalized, Math.max(0.55, Math.min(1, amplitudeExponent)));
        return centerY + (value < 0 ? -shaped : shaped) * scale;
    }

    function roundedRect(context: var, x: real, y: real, widthValue: real, heightValue: real, radiusValue: real): void {
        const radius = Math.min(Math.max(0, radiusValue), widthValue / 2, heightValue / 2);
        context.beginPath();
        context.moveTo(x + radius, y);
        context.lineTo(x + widthValue - radius, y);
        context.quadraticCurveTo(x + widthValue, y, x + widthValue, y + radius);
        context.lineTo(x + widthValue, y + heightValue - radius);
        context.quadraticCurveTo(x + widthValue, y + heightValue, x + widthValue - radius, y + heightValue);
        context.lineTo(x + radius, y + heightValue);
        context.quadraticCurveTo(x, y + heightValue, x, y + heightValue - radius);
        context.lineTo(x, y + radius);
        context.quadraticCurveTo(x, y, x + radius, y);
        context.closePath();
    }

    function traceSmoothed(context: var, values: var, first: int, last: int, peak: real, centerY: real, scale: real, reverse: bool): void {
        const count = last - first + 1;
        if (count === 1) {
            const y = displayY(values[first], peak, centerY, scale);
            context.lineTo(width, y);
            return;
        }
        const start = reverse ? last : first;
        const end = reverse ? first : last;
        const direction = reverse ? -1 : 1;
        let previousX = (start - first) / (count - 1) * width;
        let previousY = displayY(values[start], peak, centerY, scale);
        context.lineTo(previousX, previousY);
        for (let index = start + direction; reverse ? index >= end : index <= end; index += direction) {
            const x = (index - first) / (count - 1) * width;
            const y = displayY(values[index], peak, centerY, scale);
            const midpointX = (previousX + x) / 2;
            const midpointY = (previousY + y) / 2;
            context.quadraticCurveTo(previousX, previousY, midpointX, midpointY);
            previousX = x;
            previousY = y;
        }
        context.lineTo(previousX, previousY);
    }

    function drawEnvelope(context: var, mins: var, maxs: var, first: int, last: int, peak: real, centerY: real, scale: real, color: color): void {
        context.beginPath();
        traceSmoothed(context, mins, first, last, peak, centerY, scale, false);
        traceSmoothed(context, maxs, first, last, peak, centerY, scale, true);
        context.closePath();
        context.lineJoin = "round";
        context.fillStyle = color;
        context.fill();
        context.globalAlpha *= 0.34;
        context.strokeStyle = outlineColor;
        context.lineWidth = 0.75;
        context.stroke();
        context.globalAlpha /= 0.34;
    }

    function drawBars(context: var, mins: var, maxs: var, first: int, last: int, peak: real, centerY: real, scale: real, color: color): void {
        const count = last - first + 1;
        const stride = Math.max(1, barWidth + barGap);
        const barCount = Math.max(1, Math.floor((width + barGap) / stride));
        context.fillStyle = color;
        for (let barIndex = 0; barIndex < barCount; ++barIndex) {
            const sourceStart = first + Math.floor(barIndex * count / barCount);
            const sourceEnd = Math.max(sourceStart, Math.min(last, first + Math.ceil((barIndex + 1) * count / barCount) - 1));
            let minimum = 0;
            let maximum = 0;
            for (let sourceIndex = sourceStart; sourceIndex <= sourceEnd; ++sourceIndex) {
                minimum = Math.min(minimum, Number(mins[sourceIndex]));
                maximum = Math.max(maximum, Number(maxs[sourceIndex]));
            }
            const top = Math.min(displayY(minimum, peak, centerY, scale), displayY(maximum, peak, centerY, scale));
            const bottom = Math.max(displayY(minimum, peak, centerY, scale), displayY(maximum, peak, centerY, scale));
            const heightValue = Math.max(1.5, bottom - top);
            const x = barIndex * stride;
            roundedRect(context, x, centerY - heightValue / 2, Math.min(barWidth, width - x), heightValue, barRadius);
            context.fill();
        }
    }

    function drawWaveform(context: var, mins: var, maxs: var, first: int, last: int, peak: real, centerY: real, scale: real, color: color): void {
        if (renderMode === "bars") {
            drawBars(context, mins, maxs, first, last, peak, centerY, scale, color);
        } else {
            drawEnvelope(context, mins, maxs, first, last, peak, centerY, scale, color);
        }
    }

    onLevelsChanged: {
        paintReady = false;
        requestPaint();
    }
    onWidthChanged: {
        paintReady = false;
        requestPaint();
    }
    onHeightChanged: {
        paintReady = false;
        requestPaint();
    }
    onProgressChanged: requestPaint()
    onViewStartRatioChanged: {
        paintReady = false;
        requestPaint();
    }
    onViewEndRatioChanged: {
        paintReady = false;
        requestPaint();
    }
    onFillColorChanged: requestPaint()
    onProgressColorChanged: requestPaint()
    onCenterLineColorChanged: requestPaint()
    onOutlineColorChanged: requestPaint()
    onRenderModeChanged: requestPaint()
    onNormalizeChanged: requestPaint()
    onNormalizationFloorChanged: requestPaint()
    onPeakHeadroomChanged: requestPaint()
    onAmplitudeExponentChanged: requestPaint()
    onVerticalPaddingChanged: requestPaint()
    onBarWidthChanged: requestPaint()
    onBarGapChanged: requestPaint()
    onBarRadiusChanged: requestPaint()

    onPaint: {
        const context = getContext("2d");
        context.reset();
        context.clearRect(0, 0, width, height);
        if (selectedLevel < 0) {
            paintReady = true;
            return;
        }
        const level = levels[selectedLevel];
        const mins = level.mins;
        const maxs = level.maxs;
        const bucketCount = mins.length;
        if (bucketCount === 0) {
            paintReady = true;
            return;
        }

        const normalizedStart = Math.max(0, Math.min(1, viewStartRatio));
        const normalizedEnd = Math.max(normalizedStart + 0.000001, Math.min(1, viewEndRatio));
        const firstBucket = Math.min(bucketCount - 1, Math.floor(normalizedStart * bucketCount));
        const lastBucket = Math.max(firstBucket, Math.min(bucketCount - 1, Math.ceil(normalizedEnd * bucketCount) - 1));

        let peak = 0.0;
        for (let index = firstBucket; index <= lastBucket; ++index) {
            peak = Math.max(peak, Math.abs(mins[index]), Math.abs(maxs[index]));
        }
        if (peak <= 0.0)
            peak = 1.0;
        else if (normalize)
            peak = Math.max(Math.max(0.05, normalizationFloor), peak * Math.max(1, peakHeadroom));
        else
            peak = 1.0;

        const centerY = height / 2;
        const scale = Math.max(1, height / 2 - verticalPadding);

        context.strokeStyle = centerLineColor;
        context.lineWidth = 1;
        context.globalAlpha = renderMode === "bars" ? 0.26 : 0.42;
        context.beginPath();
        context.moveTo(0, centerY + 0.5);
        context.lineTo(width, centerY + 0.5);
        context.stroke();
        context.globalAlpha = renderMode === "bars" ? 0.9 : 0.88;
        drawWaveform(context, mins, maxs, firstBucket, lastBucket, peak, centerY, scale, fillColor);

        const viewProgress = (progress - normalizedStart) / Math.max(0.000001, normalizedEnd - normalizedStart);
        const playedWidth = Math.max(0, Math.min(width, viewProgress * width));
        if (playedWidth > 0) {
            context.save();
            context.beginPath();
            context.rect(0, 0, playedWidth, height);
            context.clip();
            context.globalAlpha = 1.0;
            drawWaveform(context, mins, maxs, firstBucket, lastBucket, peak, centerY, scale, progressColor);
            context.restore();
        }
        context.globalAlpha = 1.0;
        paintReady = true;
    }
}
