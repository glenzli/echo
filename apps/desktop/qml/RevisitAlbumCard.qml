//! A user-authored album cover for Revisit. The cover is made only from the
//! album's real source waveform and user facts; it never fabricates artwork.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: card

    required property var album
    property var waveformLevels: []
    property bool selected: false

    signal activated

    implicitWidth: 214
    implicitHeight: 146
    radius: Theme.cardRadius
    color: Theme.panelRaised
    border.width: selected ? 2 : 1
    border.color: selected ? Theme.accent : hover.hovered ? Theme.borderStrong : Theme.border
    clip: true

    Accessible.role: Accessible.ListItem
    Accessible.name: album.name
    Accessible.selected: selected

    function loadWaveform(): void {
        if (album.coverAsset === null || album.coverAsset.pathStatus === "missing") {
            waveformLevels = [];
            return;
        }
        try {
            waveformLevels = backend.waveformForAsset(album.coverAsset.id);
        } catch (error) {
            waveformLevels = [];
        }
    }

    Component.onCompleted: Qt.callLater(loadWaveform)
    onAlbumChanged: Qt.callLater(loadWaveform)

    HoverHandler {
        id: hover
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: card.activated()
    }

    Rectangle {
        id: cover

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 82
        color: Theme.waveformSurface

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop {
                    position: 0
                    color: Theme.effectiveDark ? "#20262c" : "#e8ecef"
                }
                GradientStop {
                    position: 1
                    color: Theme.effectiveDark ? "#2a3036" : "#f2f4f6"
                }
            }
        }

        WaveformView {
            anchors.fill: parent
            anchors.margins: 13
            levels: card.waveformLevels
            fillColor: Theme.waveformFill
            progressColor: Theme.waveformPlayed
            centerLineColor: Theme.waveformCenter
            renderMode: "bars"
            barWidth: 2.1
            barGap: 1.6
            barRadius: 1
            normalize: true
            normalizationFloor: 0.24
            amplitudeExponent: 0.78
        }

        EchoIcon {
            anchors.centerIn: parent
            visible: card.waveformLevels.length === 0
            source: "qrc:/EchoDesktop/icons/album.svg"
            size: 25
            color: Theme.textDisabled
        }

        Rectangle {
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.leftMargin: 10
            anchors.bottomMargin: 8
            width: 28
            height: 28
            radius: 8
            color: Theme.effectiveDark ? "#c51a1e23" : "#e8ffffff"

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/album.svg"
                size: 15
                color: Theme.textSecondary
            }
        }
    }

    ColumnLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: cover.bottom
        anchors.bottom: parent.bottom
        anchors.leftMargin: 13
        anchors.rightMargin: 13
        anchors.topMargin: 10
        anchors.bottomMargin: 9
        spacing: 3

        Text {
            Layout.fillWidth: true
            text: card.album.name
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
            font.bold: true
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("%1 sounds").arg(card.album.count)
            color: Theme.textMuted
            font.pixelSize: Theme.fontMeta
        }
    }
}
