//! A source-pinned clip: waveform projection, selection, move, trim and fade gestures.
import QtQuick
import QtQuick.Controls
import "SoundAssemblyEditing.js" as Editing

Rectangle {
    id: clipItem
    required property var clipData
    required property string title
    required property var sourceSpans
    required property real originalDuration
    required property real sourceDuration
    required property var waveformLevels
    required property real pixelsPerSecond
    required property bool selected
    required property color trackColor
    readonly property color waveformColor: Theme.effectiveDark ? Qt.lighter(trackColor, 1.2) : Qt.darker(trackColor, 1.3)
    required property var snapPosition
    property bool automationEditing: false
    property real viewportStart: 0
    property real viewportWidth: 1200
    property string gesture: ""
    property var draft: null
    property var initial: null
    property real groupMoveOffset: 0
    readonly property var shown: draft || (groupMoveOffset ? Object.assign({}, clipData, {timelineStartMillis: clipData.timelineStartMillis + groupMoveOffset}) : clipData)
    readonly property real duration: Editing.duration(shown)
    readonly property real visibleStart: Math.max(0, viewportStart - x)
    readonly property real visibleEnd: Math.min(width, viewportStart + viewportWidth - x)
    readonly property real visibleWidth: Math.max(0, visibleEnd - visibleStart)
    readonly property var spans: Editing.visibleSpans(sourceSpans, shown)
    signal selectedRequested(int modifiers, bool preserve)
    signal movePreviewRequested(real delta)
    signal editRequested
    signal patchRequested(var patch, string kind)
    signal contextRequested
    signal guideChanged(real position)

    objectName: "assemblyClip_" + clipData.id
    x: shown.timelineStartMillis * pixelsPerSecond / 1000
    y: 12
    width: Math.max(2, duration * pixelsPerSecond / 1000)
    height: 102
    radius: 5
    color: Qt.tint(Theme.panelRaised, Qt.alpha(trackColor, Theme.effectiveDark ? 0.22 : 0.14))
    border.width: selected ? 2 : 1
    border.color: selected ? trackColor : Qt.alpha(trackColor, 0.5)
    opacity: shown.muted ? 0.4 : 1
    z: gesture ? 5 : selected ? 3 : 1
    clip: true

    function begin(kind: string): void {
        selectedRequested(Qt.NoModifier, true);
        initial = Object.assign({}, clipData);
        draft = Object.assign({}, initial);
        gesture = kind;
    }
    function finish(): void {
        const result = draft;
        const kind = gesture;
        gesture = "";
        guideChanged(-1);
        if (result && JSON.stringify(result) !== JSON.stringify(clipData))
            patchRequested(result, kind);
        movePreviewRequested(0);
        draft = null;
    }
    function cancel(): void {
        draft = null;
        gesture = "";
        guideChanged(-1);
        movePreviewRequested(0);
    }
    function dragMove(delta: real, modifiers: int): void {
        const snapped = snapPosition(initial.timelineStartMillis + delta * 1000 / pixelsPerSecond, Editing.duration(initial), clipData.id, modifiers & Qt.ShiftModifier);
        draft = Object.assign({}, initial, {
            timelineStartMillis: snapped.position
        });
        guideChanged(snapped.guide);
        movePreviewRequested(draft.timelineStartMillis - initial.timelineStartMillis);
    }
    function dragTrim(edge: string, delta: real, modifiers: int): void {
        const raw = Editing.trim(initial, edge, delta * 1000 / pixelsPerSecond, sourceDuration);
        const endpoint = edge === "left" ? raw.timelineStartMillis : Editing.end(raw);
        const snapped = snapPosition(endpoint, 0, clipData.id, modifiers & Qt.ShiftModifier);
        const adjustment = snapped.position - endpoint;
        const next = Editing.trim(initial, edge, delta * 1000 / pixelsPerSecond + adjustment, sourceDuration);
        const actual = edge === "left" ? next.timelineStartMillis : Editing.end(next);
        draft = next;
        guideChanged(Math.abs(actual - snapped.position) < 1 ? snapped.guide : -1);
    }

    Rectangle {
        height: 26
        width: parent.width
        color: Qt.alpha(clipItem.trackColor, clipItem.selected ? 0.26 : 0.14)
    }
    Text {
        x: clipItem.visibleStart + 12
        y: 6
        width: Math.max(0, clipItem.visibleWidth - 30)
        text: clipItem.title
        color: Theme.textPrimary
        font.pixelSize: 11
        font.weight: Font.DemiBold
        elide: Text.ElideRight
    }
    Repeater {
        model: clipItem.spans
        delegate: WaveformView {
            objectName: "assemblyWaveform"
            required property var modelData
            readonly property real spanX: modelData.offset * clipItem.pixelsPerSecond / 1000
            readonly property real spanEnd: spanX + modelData.length * clipItem.pixelsPerSecond / 1000
            x: Math.max(spanX, clipItem.visibleStart)
            width: Math.max(0, Math.min(spanEnd, clipItem.visibleEnd) - x)
            y: 32
            height: 48
            visible: width > 0 && !modelData.silent
            levels: clipItem.waveformLevels
            viewStartRatio: (modelData.sourceStart + (x - spanX) * 1000 / clipItem.pixelsPerSecond) / Math.max(1, clipItem.originalDuration)
            viewEndRatio: (modelData.sourceStart + (x + width - spanX) * 1000 / clipItem.pixelsPerSecond) / Math.max(1, clipItem.originalDuration)
            fillColor: clipItem.waveformColor
            progressColor: fillColor
            outlineColor: fillColor
            normalize: true
            normalizationFloor: 0.02
            amplitudeExponent: 0.8
            centerLineColor: Qt.alpha(clipItem.trackColor, 0.3)
        }
    }
    Text {
        x: clipItem.visibleStart + 12
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 6
        width: Math.max(0, clipItem.visibleWidth - 20)
        text: (clipItem.shown.sourceRole === "material" ? qsTr("Material") : qsTr("Memory")) + "  ·  " + (clipItem.duration / 1000).toFixed(2) + " s"
        font.pixelSize: 9
        color: Theme.textSecondary
        elide: Text.ElideRight
    }
    Text {
        visible: clipItem.waveformLevels.length === 0 && clipItem.visibleWidth > 130
        x: clipItem.visibleStart + 12
        y: 47
        text: qsTr("Source waveform unavailable")
        font.pixelSize: 9
        color: Theme.textMuted
    }
    Canvas {
        id: fadeCanvas
        objectName: "assemblyFadeCurve"
        visible: clipItem.shown.fadeInMillis > 0 || clipItem.shown.fadeOutMillis > 0
        x: clipItem.visibleStart
        width: clipItem.visibleWidth
        height: clipItem.height
        onWidthChanged: requestPaint()
        onXChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            if (width <= 0)
                return;
            const incoming = clipItem.shown.fadeInMillis * clipItem.pixelsPerSecond / 1000;
            const outgoing = clipItem.shown.fadeOutMillis * clipItem.pixelsPerSecond / 1000;
            ctx.beginPath();
            let drawing = false;
            for (let pixel = 0; pixel <= width; pixel += 2) {
                const local = x + pixel;
                let gain = 1;
                if (incoming > 0 && local < incoming)
                    gain = Editing.fadeValue(local / incoming, clipItem.shown.fadeInCurve);
                if (outgoing > 0 && local > clipItem.width - outgoing)
                    gain = Editing.fadeValue((clipItem.width - local) / outgoing, clipItem.shown.fadeOutCurve);
                const y = 31 + (1 - gain) * 48;
                if (gain >= 1) {
                    if (drawing)
                        ctx.lineTo(pixel, y);
                    drawing = false;
                    continue;
                }
                if (!drawing)
                    ctx.moveTo(pixel, y);
                else
                    ctx.lineTo(pixel, y);
                drawing = true;
            }
            ctx.strokeStyle = Qt.alpha(Theme.panelRaised, 0.8);
            ctx.lineWidth = 3.5;
            ctx.stroke();
            ctx.strokeStyle = clipItem.waveformColor;
            ctx.lineWidth = 1.25;
            ctx.stroke();
        }
        Connections {
            target: clipItem
            function onShownChanged(): void {
                fadeCanvas.requestPaint();
            }
            function onPixelsPerSecondChanged(): void {
                fadeCanvas.requestPaint();
            }
            function onTrackColorChanged(): void {
                fadeCanvas.requestPaint();
            }
            function onWaveformColorChanged(): void {
                fadeCanvas.requestPaint();
            }
        }
    }
    TapHandler {
        acceptedButtons: Qt.LeftButton
        onTapped: clipItem.selectedRequested(point.modifiers, false)
        onDoubleTapped: clipItem.editRequested()
    }
    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: {
            clipItem.selectedRequested(Qt.NoModifier, true);
            clipItem.contextRequested();
        }
    }
    Item {
        x: 10
        y: 14
        width: Math.max(0, parent.width - 20)
        height: parent.height - 18
        HoverHandler {
            id: moveHover
            cursorShape: moveHandler.active ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        }
        ToolTip.visible: moveHover.hovered && !clipItem.gesture
        ToolTip.delay: 1000
        ToolTip.text: qsTr("Ctrl/Cmd-click to select multiple clips · Alt-drag to slip the source")
        DragHandler {
            id: moveHandler
            enabled: !(clipItem.automationEditing && clipItem.selected)
            objectName: "clipMoveHandle"
            target: null
            yAxis.enabled: false
            onActiveChanged: {
                if (active)
                    clipItem.begin(centroid.modifiers & Qt.AltModifier ? "slip" : "move");
                else
                    clipItem.finish();
            }
            onTranslationChanged: {
                if (active && clipItem.gesture === "slip")
                    clipItem.draft = Editing.slip(clipItem.initial, -translation.x * 1000 / clipItem.pixelsPerSecond, clipItem.sourceDuration);
                else if (active && clipItem.gesture === "move")
                    clipItem.dragMove(translation.x, centroid.modifiers);
            }
            onCanceled: clipItem.cancel()
        }
    }
    SoundGainEnvelope {
        x: clipItem.visibleStart
        y: 29
        width: clipItem.visibleWidth
        height: 53
        clipData: clipItem.shown
        pixelsPerSecond: clipItem.pixelsPerSecond
        pixelOffset: clipItem.visibleStart
        curveColor: Theme.textPrimary
        editing: clipItem.automationEditing && clipItem.selected
        onEdited: envelope => clipItem.patchRequested(Object.assign({}, clipItem.clipData, {gainEnvelope: envelope}), "envelope")
    }
    Repeater {
        model: ["left", "right"]
        delegate: Item {
            required property string modelData
            readonly property bool leftEdge: modelData === "left"
            objectName: leftEdge ? "clipTrimLeft" : "clipTrimRight"
            x: leftEdge ? 0 : clipItem.width - width
            y: 28
            width: 10
            height: clipItem.height - 30
            visible: clipItem.width >= 20
            Rectangle {
                anchors.centerIn: parent
                width: 3
                height: 23
                radius: 1
                visible: clipItem.selected || edgeHover.hovered
                color: clipItem.trackColor
            }
            HoverHandler {
                id: edgeHover
                cursorShape: Qt.SizeHorCursor
            }
            DragHandler {
                target: null
                yAxis.enabled: false
                onActiveChanged: {
                    if (active)
                        clipItem.begin(parent.modelData);
                    else
                        clipItem.finish();
                }
                onTranslationChanged: {
                    if (active && clipItem.gesture)
                        clipItem.dragTrim(parent.modelData, translation.x, centroid.modifiers);
                }
                onCanceled: clipItem.cancel()
            }
            ToolTip.visible: edgeHover.hovered && !clipItem.gesture
            ToolTip.delay: 700
            ToolTip.text: leftEdge ? qsTr("Trim start · Shift bypasses snapping") : qsTr("Trim end · Shift bypasses snapping")
        }
    }
    Repeater {
        model: ["fadeInMillis", "fadeOutMillis"]
        delegate: Item {
            required property string modelData
            readonly property bool incoming: modelData === "fadeInMillis"
            objectName: incoming ? "clipFadeIn" : "clipFadeOut"
            x: Editing.clamp(incoming ? clipItem.shown.fadeInMillis * clipItem.pixelsPerSecond / 1000 - 6 : clipItem.width - clipItem.shown.fadeOutMillis * clipItem.pixelsPerSecond / 1000 - 6, 1, clipItem.width - 13)
            y: 0
            width: 12
            height: 15
            visible: clipItem.width >= 28 && (clipItem.selected || fadeHover.hovered)
            Rectangle {
                anchors.centerIn: parent
                width: 7
                height: 7
                rotation: 45
                color: clipItem.trackColor
                border.color: Theme.panelRaised
            }
            HoverHandler {
                id: fadeHover
                cursorShape: Qt.SizeHorCursor
            }
            DragHandler {
                target: null
                yAxis.enabled: false
                onActiveChanged: {
                    if (active)
                        clipItem.begin(parent.modelData);
                    else
                        clipItem.finish();
                }
                onTranslationChanged: {
                    if (!active || !clipItem.gesture)
                        return;
                    const key = parent.modelData;
                    const other = parent.incoming ? "fadeOutMillis" : "fadeInMillis";
                    const next = Object.assign({}, clipItem.initial);
                    next[key] = Math.round(Editing.clamp(next[key] + translation.x * (parent.incoming ? 1 : -1) * 1000 / clipItem.pixelsPerSecond, 0, Editing.duration(next) - next[other]));
                    clipItem.draft = next;
                }
                onCanceled: clipItem.cancel()
            }
            ToolTip.visible: fadeHover.hovered || clipItem.gesture === modelData
            ToolTip.text: (incoming ? qsTr("Fade in") : qsTr("Fade out")) + " · " + (clipItem.shown[modelData] / 1000).toFixed(2) + " s"
        }
    }
}
