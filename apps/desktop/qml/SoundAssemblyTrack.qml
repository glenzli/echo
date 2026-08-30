//! One Sound Assembly track row: track mix controls and direct clip placement.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: trackRow

    required property var track
    required property int trackIndex
    required property real pixelsPerSecond
    required property real playheadMillis
    required property string selectedClipId
    required property bool canDeleteTrack
    property real timelineWidth: 1200

    signal trackValueRequested(int trackIndex, string key, var value)
    signal trackDeleteRequested(int trackIndex)
    signal clipSelected(int trackIndex, string clipId)
    signal clipMoveRequested(int trackIndex, string clipId, real timelineStartMillis)
    signal clipTrimRequested(int trackIndex, string clipId, real sourceStartMillis, real sourceEndMillis, real timelineStartMillis)

    property real headerWidth: 220
    implicitHeight: 118
    color: trackIndex % 2 === 0 ? Theme.panel : Theme.panelInset
    border.width: 1
    border.color: Theme.border

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: trackRow.headerWidth
        color: Theme.panelRaised

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10
            spacing: 6

            RowLayout {
                Layout.fillWidth: true

                EchoTextField {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 26
                    text: trackRow.track.name
                    maximumLength: 80
                    padding: 6
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                    onEditingFinished: {
                        const nextName = text.trim();
                        if (nextName.length > 0 && nextName !== trackRow.track.name)
                            trackRow.trackValueRequested(trackRow.trackIndex, "name", nextName);
                        else
                            text = trackRow.track.name;
                    }
                }

                Button {
                    text: qsTr("M")
                    checkable: true
                    checked: trackRow.track.muted
                    implicitWidth: 28
                    implicitHeight: 24
                    onClicked: trackRow.trackValueRequested(trackRow.trackIndex, "muted", checked)
                }

                Button {
                    text: qsTr("S")
                    checkable: true
                    checked: trackRow.track.solo
                    implicitWidth: 28
                    implicitHeight: 24
                    onClicked: trackRow.trackValueRequested(trackRow.trackIndex, "solo", checked)
                }

                Button {
                    text: "×"
                    Accessible.name: qsTr("Delete track")
                    ToolTip.visible: hovered
                    ToolTip.text: qsTr("Delete track")
                    enabled: trackRow.canDeleteTrack
                    implicitWidth: 28
                    implicitHeight: 24
                    onClicked: trackRow.trackDeleteRequested(trackRow.trackIndex)
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    text: qsTr("Gain")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontMeta
                }

                Slider {
                    Layout.fillWidth: true
                    from: -2400
                    to: 1200
                    stepSize: 50
                    value: trackRow.track.gainCentibels
                    onPressedChanged: {
                        if (!pressed)
                            trackRow.trackValueRequested(trackRow.trackIndex, "gainCentibels", Math.round(value));
                    }
                }

                Text {
                    text: (trackRow.track.gainCentibels / 100).toFixed(1)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 34
                    horizontalAlignment: Text.AlignRight
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    text: qsTr("Pan")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontMeta
                }

                Slider {
                    Layout.fillWidth: true
                    from: -100
                    to: 100
                    stepSize: 1
                    value: trackRow.track.panPercent
                    onPressedChanged: {
                        if (!pressed)
                            trackRow.trackValueRequested(trackRow.trackIndex, "panPercent", Math.round(value));
                    }
                }

                Text {
                    text: String(trackRow.track.panPercent)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    Layout.preferredWidth: 34
                    horizontalAlignment: Text.AlignRight
                }
            }
        }
    }

    Item {
        id: lane
        anchors.left: parent.left
        anchors.leftMargin: trackRow.headerWidth
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        clip: true

        Repeater {
            model: trackRow.track.clips

            delegate: Rectangle {
                id: clipBlock
                required property var modelData
                required property int index

                property real moveStartMillis: modelData.timelineStartMillis
                property real previewStartMillis: moveStartMillis
                readonly property real durationMillis: modelData.sourceEndMillis - modelData.sourceStartMillis
                readonly property bool selected: modelData.id === trackRow.selectedClipId

                x: previewStartMillis * trackRow.pixelsPerSecond / 1000
                y: 18 + (index % 2) * 6
                width: Math.max(28, durationMillis * trackRow.pixelsPerSecond / 1000)
                height: 76
                radius: 7
                color: selected ? Theme.accentSurface : Theme.surfaceSelected
                border.width: selected ? 2 : 1
                border.color: selected ? Theme.accent : Theme.borderStrong

                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: Math.max(0, Math.min(parent.width, modelData.fadeInMillis * trackRow.pixelsPerSecond / 1000))
                    color: Theme.accentSurfaceQuiet
                    opacity: 0.7
                    radius: parent.radius
                }

                Rectangle {
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: Math.max(0, Math.min(parent.width, modelData.fadeOutMillis * trackRow.pixelsPerSecond / 1000))
                    color: Theme.warningSurface
                    opacity: 0.45
                    radius: parent.radius
                }

                Column {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 4

                    Text {
                        width: parent.width
                        text: qsTr("Clip %1").arg(index + 1)
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }

                    Text {
                        width: parent.width
                        text: (clipBlock.durationMillis / 1000).toFixed(2) + qsTr(" s")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        elide: Text.ElideRight
                    }
                }

                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onTapped: trackRow.clipSelected(trackRow.trackIndex, clipBlock.modelData.id)
                }

                DragHandler {
                    id: moveHandler
                    target: null
                    xAxis.enabled: true
                    yAxis.enabled: false
                    onActiveChanged: {
                        if (active) {
                            clipBlock.moveStartMillis = clipBlock.modelData.timelineStartMillis;
                            clipBlock.previewStartMillis = clipBlock.moveStartMillis;
                            trackRow.clipSelected(trackRow.trackIndex, clipBlock.modelData.id);
                        } else {
                            trackRow.clipMoveRequested(
                                trackRow.trackIndex,
                                clipBlock.modelData.id,
                                Math.max(0, Math.round(clipBlock.previewStartMillis / 10) * 10)
                            );
                        }
                    }
                    onTranslationChanged: {
                        if (active)
                            clipBlock.previewStartMillis = Math.max(0, clipBlock.moveStartMillis + translation.x * 1000 / trackRow.pixelsPerSecond);
                    }
                }

                Rectangle {
                    id: leftTrim
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: 7
                    color: clipBlock.selected ? Theme.accent : Theme.transparent
                    opacity: trimLeftHandler.active ? 1 : 0.55

                    property real initialSourceStart: 0
                    property real initialTimelineStart: 0

                    DragHandler {
                        id: trimLeftHandler
                        target: null
                        xAxis.enabled: true
                        yAxis.enabled: false
                        onActiveChanged: {
                            if (active) {
                                leftTrim.initialSourceStart = clipBlock.modelData.sourceStartMillis;
                                leftTrim.initialTimelineStart = clipBlock.modelData.timelineStartMillis;
                                trackRow.clipSelected(trackRow.trackIndex, clipBlock.modelData.id);
                            } else {
                                const maximumDelta = clipBlock.durationMillis - 10;
                                const delta = Math.max(-leftTrim.initialSourceStart, Math.min(maximumDelta, translation.x * 1000 / trackRow.pixelsPerSecond));
                                trackRow.clipTrimRequested(
                                    trackRow.trackIndex,
                                    clipBlock.modelData.id,
                                    Math.round(leftTrim.initialSourceStart + delta),
                                    clipBlock.modelData.sourceEndMillis,
                                    Math.max(0, Math.round(leftTrim.initialTimelineStart + delta))
                                );
                            }
                        }
                    }
                }

                Rectangle {
                    id: rightTrim
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: 7
                    color: clipBlock.selected ? Theme.accent : Theme.transparent
                    opacity: trimRightHandler.active ? 1 : 0.55

                    property real initialSourceEnd: 0

                    DragHandler {
                        id: trimRightHandler
                        target: null
                        xAxis.enabled: true
                        yAxis.enabled: false
                        onActiveChanged: {
                            if (active) {
                                rightTrim.initialSourceEnd = clipBlock.modelData.sourceEndMillis;
                                trackRow.clipSelected(trackRow.trackIndex, clipBlock.modelData.id);
                            } else {
                                const minimumEnd = clipBlock.modelData.sourceStartMillis + 10;
                                const nextEnd = Math.max(minimumEnd, rightTrim.initialSourceEnd + translation.x * 1000 / trackRow.pixelsPerSecond);
                                trackRow.clipTrimRequested(
                                    trackRow.trackIndex,
                                    clipBlock.modelData.id,
                                    clipBlock.modelData.sourceStartMillis,
                                    Math.round(nextEnd),
                                    clipBlock.modelData.timelineStartMillis
                                );
                            }
                        }
                    }
                }
            }
        }

        Rectangle {
            x: trackRow.playheadMillis * trackRow.pixelsPerSecond / 1000
            width: 1
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            color: Theme.accent
            opacity: 0.8
            z: 30
        }
    }
}
