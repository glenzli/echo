//! A viewport-sized source-time gain curve; one completed gesture is one undo step.
import QtQuick
import QtQuick.Controls
import "SoundAutomation.js" as Automation

Item {
    id: surface
    required property var clipData
    required property real pixelsPerSecond
    required property real pixelOffset
    required property color curveColor
    property bool editing: false
    property var draft: null
    property int activePoint: -1
    readonly property var envelope: draft || clipData.gainEnvelope || ({enabled: false, points: []})
    readonly property var points: envelope.points || []
    readonly property real floorGain: Math.min(-2400, ...points.map(p => p.gainCentibels))
    readonly property real ceilingGain: 1200
    signal edited(var envelope)
    function timeAt(x: real): real { return Math.round(Math.max(clipData.sourceStartMillis, Math.min(clipData.sourceEndMillis, clipData.sourceStartMillis + (pixelOffset + x) * 1000 / pixelsPerSecond))); }
    function xAt(time: real): real { return (time - clipData.sourceStartMillis) * pixelsPerSecond / 1000 - pixelOffset; }
    function yAt(gain: real): real { return 5 + (ceilingGain - gain) * (height - 10) / (ceilingGain - floorGain); }
    function gainAt(y: real): real { return Math.round(Math.max(floorGain, Math.min(ceilingGain, ceilingGain - (y - 5) * (ceilingGain - floorGain) / (height - 10))) / 10) * 10; }
    function nearest(x: real, y: real): int {
        for (let i = 0; i < points.length; ++i) if (Math.abs(xAt(points[i].sourceMillis) - x) < 9 && Math.abs(yAt(points[i].gainCentibels) - y) < 10) return i;
        return -1;
    }
    onPointsChanged: curve.requestPaint()
    onPixelOffsetChanged: curve.requestPaint()
    onPixelsPerSecondChanged: curve.requestPaint()
    onCurveColorChanged: curve.requestPaint()
    onEditingChanged: { draft = null; activePoint = -1; curve.requestPaint(); }
    onClipDataChanged: { draft = null; activePoint = -1; curve.requestPaint(); }
    Canvas {
        id: curve
        anchors.fill: parent
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        opacity: surface.envelope.enabled ? 1 : 0.4
        onPaint: {
            const ctx = getContext("2d"); ctx.reset();
            if (width <= 0 || (!surface.editing && !surface.points.length)) return;
            ctx.strokeStyle = Qt.alpha(surface.curveColor, 0.3); ctx.lineWidth = 1;
            ctx.beginPath(); ctx.moveTo(0, surface.yAt(0)); ctx.lineTo(width, surface.yAt(0)); ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(0, surface.yAt(Automation.gainAt(surface.points, surface.timeAt(0))));
            for (const p of surface.points) { const x = surface.xAt(p.sourceMillis); if (x > 0 && x < width) ctx.lineTo(x, surface.yAt(p.gainCentibels)); }
            ctx.lineTo(width, surface.yAt(Automation.gainAt(surface.points, surface.timeAt(width))));
            ctx.strokeStyle = surface.curveColor; ctx.lineWidth = 2; ctx.stroke();
            if (surface.editing) for (const p of surface.points) {
                const x = surface.xAt(p.sourceMillis); if (x < -5 || x > width + 5) continue;
                ctx.beginPath(); ctx.arc(x, surface.yAt(p.gainCentibels), 4, 0, 2 * Math.PI);
                ctx.fillStyle = Theme.panelRaised; ctx.fill(); ctx.stroke();
            }
        }
    }
    MouseArea {
        id: pointer
        objectName: "gainEnvelopeHandle"
        anchors.fill: parent
        enabled: surface.editing
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        hoverEnabled: true
        cursorShape: Qt.CrossCursor
        preventStealing: true
        onPressed: mouse => {
            const index = surface.nearest(mouse.x, mouse.y);
            if (mouse.button === Qt.RightButton) {
                if (index >= 0) surface.edited({enabled: true, points: surface.points.filter((p, i) => i !== index)});
                return;
            }
            const next = index < 0 ? Automation.setPoint(surface.points, surface.timeAt(mouse.x), surface.gainAt(mouse.y)) : surface.points.slice();
            surface.activePoint = index < 0 ? next.findIndex(p => p.sourceMillis === surface.timeAt(mouse.x)) : index;
            surface.draft = {enabled: true, points: next};
        }
        onPositionChanged: mouse => {
            const index = surface.activePoint;
            if (!(pressedButtons & Qt.LeftButton) || index < 0) return;
            const next = surface.points.slice();
            const min = index ? next[index - 1].sourceMillis + 1 : surface.clipData.sourceStartMillis;
            const max = index + 1 < next.length ? next[index + 1].sourceMillis - 1 : surface.clipData.sourceEndMillis;
            next[index] = {sourceMillis: Math.max(min, Math.min(max, surface.timeAt(mouse.x))), gainCentibels: surface.gainAt(mouse.y)};
            surface.draft = {enabled: true, points: next};
        }
        onReleased: { const value = surface.draft; surface.draft = null; surface.activePoint = -1; if (value) surface.edited(value); }
        onCanceled: { surface.draft = null; surface.activePoint = -1; }
        ToolTip.visible: containsMouse && !pressed
        ToolTip.text: qsTr("Click to add a point; drag to adjust; right-click to remove.")
    }
    Text {
        visible: surface.editing
        anchors.right: parent.right
        anchors.top: parent.top
        text: surface.activePoint >= 0 && surface.activePoint < surface.points.length ? (surface.points[surface.activePoint].gainCentibels / 100).toFixed(1) + " dB" : "0 dB"
        color: surface.curveColor
        font.pixelSize: 9
    }
}
