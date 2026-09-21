//! One virtual segment in Sound Tape. It owns thumbnail waveform readiness
//! and segment-boundary presentation, never playback order or authored state.

import QtQuick
import QtQuick.Controls
import EchoDesktop

Rectangle {
    id: segment

    required property var entry
    required property bool selected
    required property real progress
    property var waveformLevels: []

    signal activated
    signal opened

    width: Math.max(disclosure.visible ? disclosure.implicitWidth+16 : 112, Math.min(330, 92 + Number(entry.durationMillis) / 1000 * 2.2))
    height: 84
    radius: 5
    color: selected ? Theme.surfaceSelected : Theme.panelRaised
    border.width: selected ? 2 : 1
    border.color: selected ? Theme.accent : Theme.border
    clip: true

    Accessible.name: titleText.text
    Accessible.role: Accessible.ListItem

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function title(): string {
        const asset = entry.asset;
        if (asset.soundCaption && asset.soundCaption.length > 0)
            return asset.soundCaption;
        if (asset.sourceTitle && asset.sourceTitle.length > 0)
            return asset.sourceTitle;
        return fileName(asset.path);
    }

    function formatDuration(millis: real): string {
        const totalSeconds = Math.max(0, Math.floor(millis / 1000));
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = totalSeconds % 60;
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds;
    }

    function loadWaveform(): void {
        waveformLevels = [];
        if (!entry || !entry.asset || !entry.asset.id)
            return;
        try {
            waveformLevels = backend.waveformForAsset(entry.asset.id);
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
        height: 56
        color: segment.selected ? Theme.accentSurfaceQuiet : Theme.waveformSurface

        WaveformView {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            levels: segment.waveformLevels
            fillColor: segment.selected ? Theme.accent : Theme.waveformFill
            progressColor: Theme.waveformPlayed
            centerLineColor: Theme.waveformCenter
            renderMode: "bars"
            barWidth: 2.2
            barGap: 1.4
            barRadius: 1.2
            normalizationFloor: 0.25
            progress: segment.progress
        }

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 2
            color: segment.selected ? Theme.accent : Theme.separatorStrong
        }
    }

    SourceDisclosureBadge { id: disclosure; asset: segment.entry.asset; anchors.left: parent.left; anchors.top: parent.top; anchors.margins: 6 }

    Text {
        id: titleText
        anchors.left: parent.left
        anchors.right: durationText.left
        anchors.bottom: parent.bottom
        anchors.leftMargin: 8
        anchors.rightMargin: 7
        height: 27
        text: segment.title()
        color: Theme.textPrimary
        font.pixelSize: 9
        font.bold: segment.selected
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    Text {
        id: durationText
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.rightMargin: 8
        height: 27
        text: segment.formatDuration(entry.durationMillis)
        color: Theme.textMuted
        font.pixelSize: 8
        verticalAlignment: Text.AlignVCenter
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: segment.activated()
        onDoubleClicked: segment.opened()
    }
}
