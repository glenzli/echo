//! Compact waveform surrogate for the single-sound filmstrip. It owns only
//! thumbnail-scale waveform readiness and selection presentation.

import QtQuick
import EchoDesktop

Rectangle {
    id: thumbnail

    required property var entry
    required property bool selected
    property var waveformLevels: []

    signal activated

    width: 152
    height: 84
    radius: 7
    color: Theme.panelRaised
    border.width: selected ? 2 : 1
    border.color: selected ? Theme.accent : Theme.border
    clip: true

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function loadWaveform(): void {
        waveformLevels = [];
        if (!entry || !entry.id || entry.pathStatus === "missing") {
            return;
        }
        try {
            waveformLevels = backend.waveformForAsset(entry.id);
        } catch (error) {
            waveformLevels = [];
        }
    }

    onEntryChanged: Qt.callLater(loadWaveform)
    Component.onCompleted: Qt.callLater(loadWaveform)

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 58
        color: thumbnail.selected ? Theme.surfaceSelected : Theme.waveformSurface

        WaveformView {
            anchors.fill: parent
            anchors.leftMargin: 7
            anchors.rightMargin: 7
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            levels: thumbnail.waveformLevels
            fillColor: thumbnail.selected ? Theme.accent : Theme.waveformFill
            progressColor: Theme.waveformPlayed
            centerLineColor: Theme.waveformCenter
            renderMode: "bars"
            barWidth: 2.2
            barGap: 1.4
            barRadius: 1.2
            normalizationFloor: 0.25
            progress: 0
        }
    }

    Text {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: 7
        anchors.rightMargin: 7
        height: 25
        text: thumbnail.entry.soundCaption.length > 0 ? thumbnail.entry.soundCaption : thumbnail.entry.sourceTitle.length > 0 ? thumbnail.entry.sourceTitle : thumbnail.fileName(thumbnail.entry.path)
        color: Theme.textPrimary
        font.pixelSize: 9
        font.bold: thumbnail.selected
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: thumbnail.activated()
    }
}
