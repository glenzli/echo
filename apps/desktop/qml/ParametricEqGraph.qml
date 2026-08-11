//! Interactive frequency-response canvas using the same C++ response
//! coefficients as playback. Presentation stays deliberately quiet so the
//! response curve and selected band carry the hierarchy.

pragma ComponentBehavior: Bound

import QtQuick
import EchoDesktop

Item {
    id: graph

    required property var draft
    required property var responseProvider
    property int selectedBand: 0
    property var response: []
    readonly property real plotTop: 12
    readonly property real plotBottom: height - 22
    readonly property real plotHeight: Math.max(1, plotBottom - plotTop)

    function frequencyX(hertz: real): real {
        return Math.log(Math.max(20, hertz) / 20) / Math.log(1000) * width;
    }

    function xFrequency(position: real): int {
        return Math.round(20 * Math.pow(1000, Math.max(0, Math.min(width, position)) / Math.max(1, width)));
    }

    function gainY(centibels: real): real {
        return plotTop + plotHeight / 2 - centibels / 1200 * (plotHeight / 2 - 8);
    }

    function yGain(position: real): int {
        return Math.round(Math.max(-1200, Math.min(1200, (plotTop + plotHeight / 2 - position) / Math.max(1, plotHeight / 2 - 8) * 1200)) / 10) * 10;
    }

    function refreshResponse(): void {
        response = responseProvider.equalizerResponse(draft.equalizerBands, 128);
        responseCanvas.requestPaint();
    }

    Canvas {
        id: responseCanvas
        anchors.fill: parent
        antialiasing: true

        onPaint: {
            const context = getContext("2d");
            context.reset();
            context.clearRect(0, 0, width, height);

            const horizontal = [-1200, -600, 0, 600, 1200];
            context.lineWidth = 1;
            for (let index = 0; index < horizontal.length; ++index) {
                const gain = horizontal[index];
                const y = graph.gainY(gain) + 0.5;
                context.strokeStyle = gain === 0 ? Theme.graphGridStrong : Theme.graphGrid;
                context.beginPath();
                context.moveTo(0, y);
                context.lineTo(width, y);
                context.stroke();
            }

            const ticks = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
            for (let index = 0; index < ticks.length; ++index) {
                const x = graph.frequencyX(ticks[index]) + 0.5;
                context.strokeStyle = [20, 100, 1000, 10000, 20000].indexOf(ticks[index]) >= 0 ? Theme.graphGridStrong : Theme.graphGrid;
                context.beginPath();
                context.moveTo(x, graph.plotTop);
                context.lineTo(x, graph.plotBottom);
                context.stroke();
            }

            context.font = "9px -apple-system, BlinkMacSystemFont, sans-serif";
            context.fillStyle = Theme.textDisabled;
            context.textBaseline = "bottom";
            const labels = [[20, "20"], [100, "100"], [1000, "1k"], [10000, "10k"], [20000, "20k"]];
            for (let index = 0; index < labels.length; ++index) {
                const x = graph.frequencyX(labels[index][0]);
                context.textAlign = index === 0 ? "left" : index === labels.length - 1 ? "right" : "center";
                context.fillText(labels[index][1], x, height);
            }

            if (graph.response.length < 2)
                return;

            context.beginPath();
            for (let index = 0; index < graph.response.length; ++index) {
                const x = index / (graph.response.length - 1) * width;
                const y = graph.gainY(Math.max(-1200, Math.min(1200, Number(graph.response[index]) * 100)));
                if (index === 0)
                    context.moveTo(x, y);
                else
                    context.lineTo(x, y);
            }
            context.lineTo(width, graph.gainY(0));
            context.lineTo(0, graph.gainY(0));
            context.closePath();
            context.fillStyle = Theme.graphFill;
            context.globalAlpha = 0.58;
            context.fill();
            context.globalAlpha = 1;

            context.lineWidth = 2.25;
            context.lineJoin = "round";
            context.strokeStyle = Theme.accent;
            context.beginPath();
            for (let index = 0; index < graph.response.length; ++index) {
                const x = index / (graph.response.length - 1) * width;
                const y = graph.gainY(Math.max(-1200, Math.min(1200, Number(graph.response[index]) * 100)));
                if (index === 0)
                    context.moveTo(x, y);
                else
                    context.lineTo(x, y);
            }
            context.stroke();
        }
    }

    Repeater {
        model: 6

        delegate: Item {
            id: node
            required property int index
            readonly property var band: graph.draft.equalizerBands[index]
            readonly property bool selected: graph.selectedBand === node.index

            x: graph.frequencyX(Number(band.frequencyHertz)) - width / 2
            y: graph.gainY(Number(band.filterKind) === 3 ? 0 : Number(band.gainCentibels)) - height / 2
            width: 28
            height: 28

            Rectangle {
                anchors.centerIn: parent
                width: node.selected ? 26 : 21
                height: width
                radius: width / 2
                color: node.selected ? Theme.accentSurface : Theme.parameterSection
                border.width: node.selected ? 2 : 1
                border.color: node.band.enabled ? Theme.accent : Theme.textDisabled

                Rectangle {
                    anchors.centerIn: parent
                    width: node.selected ? 17 : 15
                    height: width
                    radius: width / 2
                    color: node.band.enabled ? Theme.accent : Theme.panelRaised

                    Text {
                        anchors.centerIn: parent
                        text: node.index + 1
                        color: node.band.enabled ? Theme.accentText : Theme.textMuted
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                    }
                }

                Behavior on width {
                    NumberAnimation {
                        duration: 90
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                anchors.margins: -5
                hoverEnabled: true
                cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor

                onPressed: mouse => {
                    graph.selectedBand = node.index;
                    graph.draft.beginGesture();
                    updateBand(mouse);
                }
                onPositionChanged: mouse => {
                    if (pressed)
                        updateBand(mouse);
                }
                onReleased: graph.draft.endGesture()
                onCanceled: graph.draft.cancelGesture()

                function updateBand(mouse: var): void {
                    const xInGraph = node.x + mouse.x;
                    const yInGraph = node.y + mouse.y;
                    graph.draft.setEqualizerBand(node.index, true, Number(node.band.filterKind), graph.xFrequency(xInGraph), Number(node.band.qHundredths), Number(node.band.filterKind) === 3 ? Number(node.band.gainCentibels) : graph.yGain(yInGraph));
                }
            }
        }
    }

    Connections {
        target: graph.draft
        function onEqualizerBandsChanged(): void {
            graph.refreshResponse();
        }
    }

    onWidthChanged: responseCanvas.requestPaint()
    onHeightChanged: responseCanvas.requestPaint()
    Component.onCompleted: refreshResponse()
}
