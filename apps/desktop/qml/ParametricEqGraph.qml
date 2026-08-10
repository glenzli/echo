//! Interactive frequency-response editor. The plotted response comes from
//! the same C++ coefficient implementation used by playback.

pragma ComponentBehavior: Bound

import QtQuick
import EchoDesktop

Item {
    id: graph

    required property var draft
    required property var responseProvider
    property int selectedBand: 0
    property var response: []

    function frequencyX(hertz: real) : real {
        return Math.log(Math.max(20, hertz) / 20) / Math.log(1000) * width
    }

    function xFrequency(position: real) : int {
        return Math.round(20 * Math.pow(1000,
            Math.max(0, Math.min(width, position)) / Math.max(1, width)))
    }

    function gainY(centibels: real) : real {
        return height / 2 - centibels / 1200 * (height / 2 - 10)
    }

    function yGain(position: real) : int {
        return Math.round(Math.max(-1200, Math.min(1200,
            (height / 2 - position) / Math.max(1, height / 2 - 10) * 1200)) / 10) * 10
    }

    function refreshResponse() : void {
        response = responseProvider.equalizerResponse(draft.equalizerBands, 96)
        responseCanvas.requestPaint()
    }

    Canvas {
        id: responseCanvas
        anchors.fill: parent

        onPaint: {
            const context = getContext("2d")
            context.clearRect(0, 0, width, height)
            context.lineWidth = 1
            context.strokeStyle = Theme.border
            for (let gain = -1200; gain <= 1200; gain += 600) {
                const y = graph.gainY(gain)
                context.beginPath()
                context.moveTo(0, y)
                context.lineTo(width, y)
                context.stroke()
            }
            const ticks = [20, 50, 100, 200, 500, 1000, 2000, 5000,
                           10000, 20000]
            for (let index = 0; index < ticks.length; ++index) {
                const x = graph.frequencyX(ticks[index])
                context.beginPath()
                context.moveTo(x, 0)
                context.lineTo(x, height)
                context.stroke()
            }
            if (graph.response.length < 2) return
            context.lineWidth = 2
            context.strokeStyle = Theme.accent
            context.beginPath()
            for (let index = 0; index < graph.response.length; ++index) {
                const x = index / (graph.response.length - 1) * width
                const y = graph.gainY(Math.max(-1200, Math.min(1200,
                    Number(graph.response[index]) * 100)))
                if (index === 0) context.moveTo(x, y)
                else context.lineTo(x, y)
            }
            context.stroke()
        }
    }

    Repeater {
        model: 6

        delegate: Item {
            id: node
            required property int index
            readonly property var band: graph.draft.equalizerBands[index]

            x: graph.frequencyX(Number(band.frequencyHertz)) - width / 2
            y: graph.gainY(Number(band.filterKind) === 3
                ? 0 : Number(band.gainCentibels)) - height / 2
            width: 22
            height: 22

            Rectangle {
                anchors.centerIn: parent
                width: graph.selectedBand === node.index ? 18 : 15
                height: width
                radius: width / 2
                color: node.band.enabled ? Theme.accent : Theme.panelRaised
                border.width: 2
                border.color: node.band.enabled ? Theme.accent : Theme.textDisabled

                Text {
                    anchors.centerIn: parent
                    text: node.index + 1
                    color: node.band.enabled ? "white" : Theme.textSecondary
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                }
            }

            MouseArea {
                anchors.fill: parent
                anchors.margins: -6
                hoverEnabled: true

                onPressed: mouse => {
                    graph.selectedBand = node.index
                    graph.draft.beginGesture()
                    updateBand(mouse)
                }
                onPositionChanged: mouse => {
                    if (pressed) updateBand(mouse)
                }
                onReleased: graph.draft.endGesture()
                onCanceled: graph.draft.cancelGesture()

                function updateBand(mouse: var) : void {
                    const xInGraph = node.x + mouse.x
                    const yInGraph = node.y + mouse.y
                    graph.draft.setEqualizerBand(node.index, true,
                        Number(node.band.filterKind),
                        graph.xFrequency(xInGraph),
                        Number(node.band.qHundredths),
                        Number(node.band.filterKind) === 3
                            ? Number(node.band.gainCentibels)
                            : graph.yGain(yInGraph))
                }
            }
        }
    }

    Connections {
        target: graph.draft
        function onEqualizerBandsChanged() : void {
            graph.refreshResponse()
        }
    }

    onWidthChanged: responseCanvas.requestPaint()
    onHeightChanged: responseCanvas.requestPaint()
    Component.onCompleted: refreshResponse()
}
