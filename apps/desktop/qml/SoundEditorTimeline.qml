//! Direct-manipulation owner for one adjustment draft. Viewport state,
//! timeline projection, pointer gestures, and their transient readouts stay
//! here; persistence and playback lifecycle remain in SoundEditingWorkspace.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: timeline

    required property var waveformLevels
    required property int sourceDurationMillis
    required property int trimStartMillis
    required property int trimEndMillis
    required property int fadeInMillis
    required property int fadeOutMillis
    required property int gainCentibels
    required property int playbackPositionMillis
    required property bool isPlaying

    property real zoomFactor: 1.0
    property real viewStartMillis: 0.0
    property bool followPlayhead: true
    property string activeGesture: ""
    property string gestureReadout: ""

    readonly property real maximumZoomFactor: Math.max(1,
        Math.min(4096, sourceDurationMillis / 1000))
    readonly property real viewDurationMillis: sourceDurationMillis > 0
        ? sourceDurationMillis / Math.max(1, zoomFactor) : 1
    readonly property real viewEndMillis: Math.min(sourceDurationMillis,
        viewStartMillis + viewDurationMillis)
    readonly property real maximumViewStartMillis: Math.max(0,
        sourceDurationMillis - viewDurationMillis)
    readonly property int minimumSelectionMillis: Math.min(1000,
        Math.max(50, Math.round(sourceDurationMillis / 10000)))
    readonly property real viewStartRatio: sourceDurationMillis > 0
        ? viewStartMillis / sourceDurationMillis : 0
    readonly property real viewEndRatio: sourceDurationMillis > 0
        ? viewEndMillis / sourceDurationMillis : 1
    readonly property int tickIntervalMillis: chooseTickInterval()
    readonly property int firstTickMillis: tickIntervalMillis > 0
        ? Math.ceil(viewStartMillis / tickIntervalMillis) * tickIntervalMillis : 0
    readonly property int tickCount: tickIntervalMillis > 0
        ? Math.ceil(viewDurationMillis / tickIntervalMillis) + 2 : 0

    signal trimRequested(int startMillis, int endMillis)
    signal fadeRequested(int fadeInMillis, int fadeOutMillis)
    signal gainRequested(int gainCentibels)
    signal seekRequested(int millis)
    signal playPauseRequested()

    implicitHeight: 440
    color: Theme.panelRaised
    radius: Theme.panelRadius
    border.color: Theme.border
    clip: true
    focus: true

    function clamp(value: real, minimum: real, maximum: real) : real {
        return Math.max(minimum, Math.min(maximum, value))
    }

    function formatTime(millis: real, detailed: bool) : string {
        const safeMillis = Math.max(0, Math.round(millis))
        const totalSeconds = Math.floor(safeMillis / 1000)
        const hours = Math.floor(totalSeconds / 3600)
        const minutes = Math.floor((totalSeconds % 3600) / 60)
        const seconds = totalSeconds % 60
        const tenths = Math.floor((safeMillis % 1000) / 100)
        if (hours > 0) {
            return hours + ":" + String(minutes).padStart(2, "0") + ":"
                + String(seconds).padStart(2, "0")
                + (detailed ? "." + tenths : "")
        }
        return minutes + ":" + String(seconds).padStart(2, "0")
            + (detailed ? "." + tenths : "")
    }

    function formatGain(centibels: int) : string {
        return (centibels >= 0 ? "+" : "")
            + (centibels / 100).toFixed(1) + " dB"
    }

    function chooseTickInterval() : int {
        const candidates = [100, 250, 500, 1000, 2000, 5000, 10000,
                            15000, 30000, 60000, 120000, 300000, 600000,
                            1800000, 3600000]
        const target = viewDurationMillis / Math.max(2, trackSurface.width / 90)
        for (let index = 0; index < candidates.length; ++index) {
            if (candidates[index] >= target) {
                return candidates[index]
            }
        }
        return candidates[candidates.length - 1]
    }

    function timeToX(millis: real) : real {
        if (trackSurface.width <= 0 || viewDurationMillis <= 0) {
            return 0
        }
        return (millis - viewStartMillis) / viewDurationMillis * trackSurface.width
    }

    function xToTime(x: real) : int {
        if (trackSurface.width <= 0) {
            return 0
        }
        return Math.round(clamp(viewStartMillis
            + x / trackSurface.width * viewDurationMillis,
            0, sourceDurationMillis))
    }

    function gainToY(centibels: int) : real {
        const top = 22
        const bottom = Math.max(top + 1, trackSurface.height - 22)
        const normalized = (1200 - clamp(centibels, -2400, 1200)) / 3600
        return top + normalized * (bottom - top)
    }

    function yToGain(y: real) : int {
        const top = 22
        const bottom = Math.max(top + 1, trackSurface.height - 22)
        const normalized = clamp((y - top) / (bottom - top), 0, 1)
        return Math.round((1200 - normalized * 3600) / 50) * 50
    }

    function setViewStart(millis: real, manual: bool) : void {
        viewStartMillis = clamp(millis, 0, maximumViewStartMillis)
        if (manual) {
            followPlayhead = false
        }
    }

    function setZoomAround(anchorX: real, requestedZoom: real, manual: bool) : void {
        if (sourceDurationMillis <= 0 || trackSurface.width <= 0) {
            return
        }
        const anchorRatio = clamp(anchorX / trackSurface.width, 0, 1)
        const anchorTime = viewStartMillis + anchorRatio * viewDurationMillis
        zoomFactor = clamp(requestedZoom, 1, maximumZoomFactor)
        const nextDuration = sourceDurationMillis / zoomFactor
        viewStartMillis = clamp(anchorTime - anchorRatio * nextDuration,
                                0, Math.max(0, sourceDurationMillis - nextDuration))
        if (manual) {
            followPlayhead = false
        }
    }

    function fitAll() : void {
        zoomFactor = 1
        viewStartMillis = 0
        followPlayhead = true
    }

    function fitSelection() : void {
        if (sourceDurationMillis <= 0 || trimEndMillis <= trimStartMillis) {
            return
        }
        const selected = trimEndMillis - trimStartMillis
        const paddedDuration = Math.min(sourceDurationMillis, selected * 1.18)
        zoomFactor = clamp(sourceDurationMillis / Math.max(1, paddedDuration),
                           1, maximumZoomFactor)
        const actualDuration = sourceDurationMillis / zoomFactor
        viewStartMillis = clamp(trimStartMillis - (actualDuration - selected) / 2,
                                0, Math.max(0, sourceDurationMillis - actualDuration))
        followPlayhead = false
    }

    function scrollBy(deltaMillis: real, manual: bool) : void {
        setViewStart(viewStartMillis + deltaMillis, manual)
    }

    function beginGesture(kind: string, readout: string) : void {
        activeGesture = kind
        gestureReadout = readout
        forceActiveFocus()
    }

    function endGesture() : void {
        activeGesture = ""
        gestureReadout = ""
    }

    onSourceDurationMillisChanged: fitAll()
    onTrimStartMillisChanged: envelopeCanvas.requestPaint()
    onTrimEndMillisChanged: envelopeCanvas.requestPaint()
    onFadeInMillisChanged: envelopeCanvas.requestPaint()
    onFadeOutMillisChanged: envelopeCanvas.requestPaint()
    onGainCentibelsChanged: envelopeCanvas.requestPaint()
    onViewStartMillisChanged: envelopeCanvas.requestPaint()
    onViewDurationMillisChanged: envelopeCanvas.requestPaint()
    onPlaybackPositionMillisChanged: {
        if (!isPlaying || !followPlayhead || zoomFactor <= 1) {
            return
        }
        if (playbackPositionMillis >= viewEndMillis) {
            setViewStart(playbackPositionMillis - viewDurationMillis * 0.12, false)
        } else if (playbackPositionMillis < viewStartMillis) {
            setViewStart(playbackPositionMillis, false)
        }
    }

    Keys.onSpacePressed: timeline.playPauseRequested()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            spacing: 8

            Text {
                text: qsTr("Timeline")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.bold: true
            }

            Text {
                text: timeline.formatTime(timeline.viewStartMillis, true) + " — "
                    + timeline.formatTime(timeline.viewEndMillis, true)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Follow")
                ghost: true
                selected: timeline.followPlayhead
                onClicked: timeline.followPlayhead = !timeline.followPlayhead
            }

            EchoButton {
                text: qsTr("Fit selection")
                ghost: true
                enabled: timeline.trimEndMillis > timeline.trimStartMillis
                onClicked: timeline.fitSelection()
            }

            EchoButton {
                text: qsTr("Fit all")
                ghost: true
                onClicked: timeline.fitAll()
            }

            Text {
                text: qsTr("Zoom")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Slider {
                id: zoomSlider

                Layout.preferredWidth: 150
                from: 0
                to: Math.log(Math.max(1, timeline.maximumZoomFactor)) / Math.LN2
                value: Math.log(Math.max(1, timeline.zoomFactor)) / Math.LN2
                enabled: timeline.maximumZoomFactor > 1
                Accessible.name: qsTr("Timeline zoom")
                onMoved: timeline.setZoomAround(trackSurface.width / 2,
                    Math.pow(2, value), true)
            }

            Text {
                text: Math.round(timeline.zoomFactor * 10) / 10 + "×"
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 42
                horizontalAlignment: Text.AlignRight
            }
        }

        Rectangle {
            id: ruler

            Layout.fillWidth: true
            Layout.preferredHeight: 30
            color: Theme.surfaceSubtle
            border.color: Theme.border
            clip: true

            Repeater {
                model: timeline.tickCount

                delegate: Item {
                    required property int index

                    property int tickMillis: timeline.firstTickMillis
                        + index * timeline.tickIntervalMillis
                    x: timeline.timeToX(tickMillis)
                    width: 1
                    height: ruler.height
                    visible: tickMillis >= timeline.viewStartMillis
                        && tickMillis <= timeline.viewEndMillis

                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: 1
                        height: 8
                        color: Theme.separatorStrong
                    }

                    Text {
                        x: 5
                        y: 4
                        text: timeline.formatTime(parent.tickMillis,
                            timeline.tickIntervalMillis < 1000)
                        color: Theme.textSecondary
                        font.pixelSize: 9
                    }
                }
            }
        }

        Rectangle {
            id: trackSurface

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 250
            color: Theme.waveformSurface
            clip: true

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                y: timeline.gainToY(0)
                height: 1
                color: Theme.borderStrong
                opacity: 0.7
            }

            Text {
                anchors.right: parent.right
                anchors.rightMargin: 6
                y: timeline.gainToY(0) - height - 3
                text: "0 dB"
                color: Theme.textDisabled
                font.pixelSize: 9
            }

            WaveformView {
                anchors.fill: parent
                anchors.margins: 14
                levels: timeline.waveformLevels
                viewStartRatio: timeline.viewStartRatio
                viewEndRatio: timeline.viewEndRatio
                progress: timeline.sourceDurationMillis > 0
                    ? timeline.playbackPositionMillis / timeline.sourceDurationMillis : 0
                fillColor: Theme.textSecondary
                progressColor: Theme.accent
                opacity: 0.78
            }

            MouseArea {
                id: trackInput

                anchors.fill: parent
                z: 1
                enabled: timeline.enabled
                acceptedButtons: Qt.LeftButton
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onPressed: function(mouse) {
                    timeline.forceActiveFocus()
                    timeline.followPlayhead = true
                    timeline.seekRequested(timeline.xToTime(mouse.x))
                }
                onWheel: function(wheel) {
                    const horizontal = Math.abs(wheel.pixelDelta.x) > Math.abs(wheel.pixelDelta.y)
                    const scrollModifier = (wheel.modifiers & Qt.ShiftModifier) !== 0
                    if (horizontal || scrollModifier) {
                        const pixels = horizontal ? wheel.pixelDelta.x
                            : wheel.pixelDelta.y !== 0 ? wheel.pixelDelta.y
                            : wheel.angleDelta.y / 2
                        timeline.scrollBy(-pixels / Math.max(1, width)
                            * timeline.viewDurationMillis, true)
                    } else {
                        const delta = wheel.pixelDelta.y !== 0
                            ? wheel.pixelDelta.y : wheel.angleDelta.y
                        timeline.setZoomAround(wheel.x,
                            timeline.zoomFactor * Math.pow(1.0025, delta), true)
                    }
                    wheel.accepted = true
                }
            }

            Rectangle {
                z: 2
                x: 0
                y: 0
                width: Math.max(0, timeline.timeToX(timeline.trimStartMillis))
                height: parent.height
                color: Theme.window
                opacity: 0.74
            }

            Rectangle {
                z: 2
                x: Math.max(0, timeline.timeToX(timeline.trimEndMillis))
                y: 0
                width: Math.max(0, parent.width - x)
                height: parent.height
                color: Theme.window
                opacity: 0.74
            }

            Canvas {
                id: envelopeCanvas

                anchors.fill: parent
                z: 3
                antialiasing: true
                renderStrategy: Canvas.Immediate

                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onPaint: {
                    const context = getContext("2d")
                    context.reset()
                    context.clearRect(0, 0, width, height)
                    if (timeline.sourceDurationMillis <= 0) {
                        return
                    }

                    const startX = timeline.timeToX(timeline.trimStartMillis)
                    const endX = timeline.timeToX(timeline.trimEndMillis)
                    const fadeInX = timeline.timeToX(timeline.trimStartMillis
                        + timeline.fadeInMillis)
                    const fadeOutX = timeline.timeToX(timeline.trimEndMillis
                        - timeline.fadeOutMillis)
                    const gainY = timeline.gainToY(timeline.gainCentibels)
                    const silenceY = height - 18

                    context.strokeStyle = Theme.accent
                    context.lineWidth = 2
                    context.beginPath()
                    context.moveTo(startX, timeline.fadeInMillis > 0 ? silenceY : gainY)
                    context.lineTo(fadeInX, gainY)
                    context.lineTo(fadeOutX, gainY)
                    context.lineTo(endX, timeline.fadeOutMillis > 0 ? silenceY : gainY)
                    context.stroke()

                    context.fillStyle = Theme.accent
                    context.globalAlpha = 0.08
                    context.beginPath()
                    context.moveTo(startX, silenceY)
                    context.lineTo(startX, timeline.fadeInMillis > 0 ? silenceY : gainY)
                    context.lineTo(fadeInX, gainY)
                    context.lineTo(fadeOutX, gainY)
                    context.lineTo(endX, timeline.fadeOutMillis > 0 ? silenceY : gainY)
                    context.lineTo(endX, silenceY)
                    context.closePath()
                    context.fill()
                    context.globalAlpha = 1
                }
            }

            MouseArea {
                id: gainGesture

                z: 5
                x: Math.max(0, timeline.timeToX(timeline.trimStartMillis
                    + timeline.fadeInMillis))
                y: timeline.gainToY(timeline.gainCentibels) - 10
                width: Math.max(0, Math.min(trackSurface.width,
                    timeline.timeToX(timeline.trimEndMillis - timeline.fadeOutMillis)) - x)
                height: 20
                enabled: timeline.enabled && width > 20
                hoverEnabled: true
                cursorShape: Qt.SizeVerCursor
                onPressed: function(mouse) {
                    timeline.beginGesture("gain", timeline.formatGain(timeline.gainCentibels))
                }
                onPositionChanged: function(mouse) {
                    if (!pressed) return
                    const point = mapToItem(trackSurface, mouse.x, mouse.y)
                    const gain = timeline.yToGain(point.y)
                    timeline.gainRequested(gain)
                    timeline.gestureReadout = timeline.formatGain(gain)
                }
                onReleased: timeline.endGesture()
                onCanceled: timeline.endGesture()
            }

            Rectangle {
                id: trimInHandle

                z: 7
                x: timeline.timeToX(timeline.trimStartMillis) - width / 2
                width: 12
                height: trackSurface.height
                color: Theme.accentSurface
                border.color: Theme.accent
                visible: x + width >= 0 && x <= trackSurface.width

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 5
                    height: 42
                    radius: 2
                    color: Theme.accent
                }

                MouseArea {
                    anchors.fill: parent
                    enabled: timeline.enabled
                    hoverEnabled: true
                    cursorShape: Qt.SizeHorCursor
                    onPressed: timeline.beginGesture("trim",
                        timeline.formatTime(timeline.trimStartMillis, true))
                    onPositionChanged: function(mouse) {
                        if (!pressed) return
                        const point = mapToItem(trackSurface, mouse.x, mouse.y)
                        const value = Math.min(timeline.xToTime(point.x),
                            timeline.trimEndMillis - timeline.minimumSelectionMillis)
                        timeline.trimRequested(value, timeline.trimEndMillis)
                        timeline.gestureReadout = timeline.formatTime(value, true)
                    }
                    onReleased: timeline.endGesture()
                    onCanceled: timeline.endGesture()
                }
            }

            Rectangle {
                id: trimOutHandle

                z: 7
                x: timeline.timeToX(timeline.trimEndMillis) - width / 2
                width: 12
                height: trackSurface.height
                color: Theme.accentSurface
                border.color: Theme.accent
                visible: x + width >= 0 && x <= trackSurface.width

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 5
                    height: 42
                    radius: 2
                    color: Theme.accent
                }

                MouseArea {
                    anchors.fill: parent
                    enabled: timeline.enabled
                    hoverEnabled: true
                    cursorShape: Qt.SizeHorCursor
                    onPressed: timeline.beginGesture("trim",
                        timeline.formatTime(timeline.trimEndMillis, true))
                    onPositionChanged: function(mouse) {
                        if (!pressed) return
                        const point = mapToItem(trackSurface, mouse.x, mouse.y)
                        const value = Math.max(timeline.xToTime(point.x),
                            timeline.trimStartMillis + timeline.minimumSelectionMillis)
                        timeline.trimRequested(timeline.trimStartMillis, value)
                        timeline.gestureReadout = timeline.formatTime(value, true)
                    }
                    onReleased: timeline.endGesture()
                    onCanceled: timeline.endGesture()
                }
            }

            Rectangle {
                id: fadeInHandle

                z: 8
                x: timeline.timeToX(timeline.trimStartMillis
                    + timeline.fadeInMillis) - width / 2
                y: timeline.gainToY(timeline.gainCentibels) - height / 2
                width: 14
                height: 14
                radius: 7
                color: Theme.panelRaised
                border.width: 2
                border.color: Theme.accent
                visible: x + width >= 0 && x <= trackSurface.width

                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -7
                    enabled: timeline.enabled
                    hoverEnabled: true
                    cursorShape: Qt.SizeHorCursor
                    onPressed: timeline.beginGesture("fade",
                        qsTr("Fade in %1").arg(timeline.formatTime(
                            timeline.fadeInMillis, true)))
                    onPositionChanged: function(mouse) {
                        if (!pressed) return
                        const point = mapToItem(trackSurface, mouse.x, mouse.y)
                        const maximum = Math.max(0, timeline.trimEndMillis
                            - timeline.trimStartMillis - timeline.fadeOutMillis)
                        const value = Math.round(timeline.clamp(
                            timeline.xToTime(point.x) - timeline.trimStartMillis,
                            0, maximum) / 50) * 50
                        timeline.fadeRequested(value, timeline.fadeOutMillis)
                        timeline.gestureReadout = qsTr("Fade in %1").arg(
                            timeline.formatTime(value, true))
                    }
                    onReleased: timeline.endGesture()
                    onCanceled: timeline.endGesture()
                }
            }

            Rectangle {
                id: fadeOutHandle

                z: 8
                x: timeline.timeToX(timeline.trimEndMillis
                    - timeline.fadeOutMillis) - width / 2
                y: timeline.gainToY(timeline.gainCentibels) - height / 2
                width: 14
                height: 14
                radius: 7
                color: Theme.panelRaised
                border.width: 2
                border.color: Theme.accent
                visible: x + width >= 0 && x <= trackSurface.width

                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -7
                    enabled: timeline.enabled
                    hoverEnabled: true
                    cursorShape: Qt.SizeHorCursor
                    onPressed: timeline.beginGesture("fade",
                        qsTr("Fade out %1").arg(timeline.formatTime(
                            timeline.fadeOutMillis, true)))
                    onPositionChanged: function(mouse) {
                        if (!pressed) return
                        const point = mapToItem(trackSurface, mouse.x, mouse.y)
                        const maximum = Math.max(0, timeline.trimEndMillis
                            - timeline.trimStartMillis - timeline.fadeInMillis)
                        const value = Math.round(timeline.clamp(
                            timeline.trimEndMillis - timeline.xToTime(point.x),
                            0, maximum) / 50) * 50
                        timeline.fadeRequested(timeline.fadeInMillis, value)
                        timeline.gestureReadout = qsTr("Fade out %1").arg(
                            timeline.formatTime(value, true))
                    }
                    onReleased: timeline.endGesture()
                    onCanceled: timeline.endGesture()
                }
            }

            Rectangle {
                z: 9
                x: timeline.timeToX(timeline.playbackPositionMillis)
                width: 1
                height: parent.height
                color: Theme.textPrimary
                visible: x >= 0 && x <= parent.width

                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 9
                    height: 7
                    color: Theme.textPrimary
                }
            }

            Rectangle {
                z: 10
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 12
                width: gestureText.implicitWidth + 18
                height: 28
                radius: 6
                color: Theme.chrome
                border.color: Theme.borderStrong
                visible: timeline.activeGesture.length > 0

                Text {
                    id: gestureText
                    anchors.centerIn: parent
                    text: timeline.gestureReadout
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMeta
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 42
            spacing: 10

            Text {
                text: timeline.formatTime(timeline.viewStartMillis, true)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 54
            }

            Slider {
                id: navigator

                Layout.fillWidth: true
                from: 0
                to: Math.max(1, timeline.maximumViewStartMillis)
                value: timeline.viewStartMillis
                enabled: timeline.zoomFactor > 1
                Accessible.name: qsTr("Timeline position")
                onMoved: timeline.setViewStart(value, true)
            }

            Text {
                text: timeline.formatTime(timeline.viewEndMillis, true)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                Layout.preferredWidth: 54
                horizontalAlignment: Text.AlignRight
            }

            Rectangle {
                width: selectionText.implicitWidth + 18
                height: 26
                radius: 6
                color: Theme.surfaceSubtle
                border.color: Theme.border

                Text {
                    id: selectionText
                    anchors.centerIn: parent
                    text: qsTr("Selection %1").arg(timeline.formatTime(
                        timeline.trimEndMillis - timeline.trimStartMillis, true))
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }
        }
    }
}
