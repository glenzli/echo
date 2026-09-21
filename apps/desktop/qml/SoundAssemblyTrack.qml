//! One arrangement lane and its fixed, compact track controls.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SoundAssemblyEditing.js" as Editing

Rectangle {
    id: trackRow
    required property var track
    required property int trackIndex
    required property real pixelsPerSecond
    required property real playheadMillis
    property var selectedClipIds: [selectedClipId]
    property string groupMoveId: ""
    property real groupMoveDelta: 0
    signal movePreviewRequested(string clipId, real delta)
    required property string selectedClipId
    required property bool canDeleteTrack
    property real timelineWidth: 1200
    property real headerWidth: 208
    property real horizontalOffset: 0
    property real viewportWidth: 1200
    property var sourceAssets: []
    property var clipSources: []
    property var waveforms: ({})
    property var snapPosition
    property bool anySolo: false
    property bool automationEditing: false
    property var meterPeaks: ({})
    readonly property color trackColor: Theme.assemblyTrackColors[trackIndex % 8]
    readonly property var crossfades: {
        const pairs = [];
        for (let a = 0; a < track.clips.length; ++a)
            for (let b = a + 1; b < track.clips.length; ++b) {
                const pair = Editing.crossfade(track.clips[a], track.clips[b]);
                if (!pair)
                    continue;
                const first = track.clips.find(clip => clip.id === pair.first), second = track.clips.find(clip => clip.id === pair.second);
                if (first.fadeOutMillis === pair.duration && second.fadeInMillis === pair.duration)
                    pairs.push({
                        start: second.timelineStartMillis,
                        duration: pair.duration,
                        outgoing: first.fadeOutCurve,
                        incoming: second.fadeInCurve
                    });
            }
        return pairs;
    }
    signal sourceDropped(var asset, int trackIndex, real positionMillis)
    signal clipEditRequested(int trackIndex, string clipId)
    signal trackValueRequested(int trackIndex, string key, var value)
    signal trackPreviewRequested(int trackIndex, string key, var value)
    signal trackMixResetRequested(int trackIndex)
    signal trackDeleteRequested(int trackIndex)
    signal clipSelected(int trackIndex, string clipId, int modifiers, bool preserve)
    signal clipPatchRequested(string clipId, var patch, string kind)
    signal contextRequested
    signal guideChanged(real position)
    signal seekRequested(real position)

    implicitHeight: 130
    color: trackIndex % 2 === 0 ? Theme.panel : Theme.panelInset
    border.width: 1
    border.color: Theme.border

    function sourceFor(clip: var): var {
        return Editing.pinnedSource(clip, clipSources, sourceAssets);
    }
    function assetFor(clip: var): var {
        return sourceAssets.find(source => source.id === clip.assetId) || ({});
    }
    function spansFor(clip: var): var {
        const source = sourceFor(clip);
        return source === null ? [] : Editing.sourceSpans(source, Number(assetFor(clip).durationMillis || 0));
    }

    Item {
        id: lane
        x: trackRow.headerWidth
        width: parent.width - x
        height: parent.height
        clip: true
        TapHandler {
            onTapped: eventPoint => trackRow.seekRequested(Math.max(0, eventPoint.position.x * 1000 / trackRow.pixelsPerSecond))
        }
        DropArea {
            anchors.fill: parent
            keys: ["echo-sound"]
            onDropped: drop => {
                if (drop.source && drop.source.asset) {
                    trackRow.sourceDropped(drop.source.asset, trackRow.trackIndex, Math.max(0, drop.x * 1000 / trackRow.pixelsPerSecond));
                    drop.acceptProposedAction();
                }
            }
        }
        Repeater {
            model: trackRow.track.clips
            delegate: SoundAssemblyClip {
                required property var modelData
                clipData: modelData
                automationEditing: trackRow.automationEditing
                title: {
                    const source = trackRow.assetFor(modelData);
                    return SoundSemantics.sourceTitle(source) || qsTr("Unavailable source");
                }
                sourceSpans: trackRow.spansFor(modelData)
                sourceDuration: sourceSpans.length ? sourceSpans[sourceSpans.length - 1].end : modelData.sourceEndMillis
                originalDuration: Number(trackRow.assetFor(modelData).durationMillis || sourceDuration)
                waveformLevels: trackRow.waveforms[modelData.assetId] || []
                pixelsPerSecond: trackRow.pixelsPerSecond
                selected: trackRow.selectedClipIds.includes(modelData.id)
                groupMoveOffset: selected && trackRow.groupMoveId && trackRow.groupMoveId !== modelData.id ? trackRow.groupMoveDelta : 0
                onMovePreviewRequested: delta => trackRow.movePreviewRequested(delta === 0 ? "" : modelData.id, delta)
                trackColor: trackRow.trackColor
                snapPosition: trackRow.snapPosition
                viewportStart: trackRow.horizontalOffset
                viewportWidth: trackRow.viewportWidth - trackRow.headerWidth
                opacity: modelData.muted || trackRow.track.muted || (trackRow.anySolo && !trackRow.track.solo) ? 0.38 : 1
                onSelectedRequested: (modifiers, preserve) => trackRow.clipSelected(trackRow.trackIndex, modelData.id, modifiers, preserve)
                onEditRequested: trackRow.clipEditRequested(trackRow.trackIndex, modelData.id)
                onPatchRequested: (patch, kind) => trackRow.clipPatchRequested(modelData.id, patch, kind)
                onContextRequested: trackRow.contextRequested()
                onGuideChanged: position => trackRow.guideChanged(position)
            }
        }
        Repeater {
            model: trackRow.crossfades
            delegate: Canvas {
                required property var modelData
                readonly property real start: modelData.start * trackRow.pixelsPerSecond / 1000
                readonly property real span: modelData.duration * trackRow.pixelsPerSecond / 1000
                x: Math.max(start, trackRow.horizontalOffset)
                y: 43
                width: Math.max(0, Math.min(start + span, trackRow.horizontalOffset + trackRow.viewportWidth - trackRow.headerWidth) - x)
                height: 48
                z: 6
                onWidthChanged: requestPaint()
                onXChanged: requestPaint()
                onModelDataChanged: requestPaint()
                onPaint: {
                    const ctx = getContext("2d");
                    ctx.reset();
                    ctx.fillStyle = Qt.alpha(trackRow.trackColor, 0.09);
                    ctx.fillRect(0, 0, width, height);
                    ctx.strokeStyle = trackRow.trackColor;
                    ctx.lineWidth = 1.4;
                    for (const incoming of [false, true]) {
                        ctx.beginPath();
                        for (let pixel = 0; pixel <= width; pixel += 2) {
                            const t = (x + pixel - start) / span;
                            const value = Editing.fadeValue(incoming ? t : 1 - t, incoming ? modelData.incoming : modelData.outgoing);
                            if (pixel === 0)
                                ctx.moveTo(pixel, (1 - value) * height);
                            else
                                ctx.lineTo(pixel, (1 - value) * height);
                        }
                        ctx.stroke();
                    }
                }
            }
        }
    }

    // Counter-scroll the header while retaining the same vertical track geometry.
    Rectangle {
        objectName: "assemblyTrackHeader_" + trackRow.trackIndex
        x: trackRow.horizontalOffset
        width: trackRow.headerWidth
        height: parent.height
        color: Theme.panelRaised
        z: 10
        Rectangle {
            width: 3
            height: parent.height
            color: trackRow.trackColor
        }
        Rectangle {
            anchors.right: parent.right
            width: 1
            height: parent.height
            color: Theme.borderStrong
        }
        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton | Qt.RightButton
            onPressed: mouse => mouse.accepted = true
        }
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 7
            RowLayout {
                spacing: 6
                Text {
                    text: String(trackRow.trackIndex + 1).padStart(2, "0")
                    color: trackRow.trackColor
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                }
                EchoTextField {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 24
                    text: trackRow.track.name
                    maximumLength: 80
                    padding: 3
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                    onEditingFinished: {
                        if (text.trim().length > 0 && text.trim() !== trackRow.track.name)
                            trackRow.trackValueRequested(trackRow.trackIndex, "name", text.trim());
                        else
                            text = trackRow.track.name;
                    }
                }
                EchoIconButton {
                    buttonSize: 22
                    iconSize: 14
                    source: "qrc:/EchoDesktop/icons/more-horizontal.svg"
                    toolTipText: qsTr("Track actions")
                    onClicked: trackMenu.popup()
                }
            }
            RowLayout {
                spacing: 5
                Repeater {
                    model: [
                        {
                            key: "muted",
                            label: qsTr("M"),
                            hint: qsTr("Mute track")
                        },
                        {
                            key: "solo",
                            label: qsTr("S"),
                            hint: qsTr("Solo track")
                        }
                    ]
                    delegate: Button {
                        required property var modelData
                        implicitWidth: 27
                        implicitHeight: 24
                        text: modelData.label
                        checkable: true
                        checked: trackRow.track[modelData.key]
                        onClicked: trackRow.trackValueRequested(trackRow.trackIndex, modelData.key, checked)
                        background: Rectangle {
                            radius: 4
                            color: parent.checked ? Qt.alpha(trackRow.trackColor, 0.24) : Theme.controlQuiet
                            border.width: 1
                            border.color: parent.checked ? trackRow.trackColor : Theme.border
                        }
                        contentItem: Text {
                            text: parent.text
                            color: Theme.textPrimary
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        ToolTip.visible: hovered
                        ToolTip.text: modelData.hint
                        Accessible.name: modelData.hint
                    }
                }
                EchoStereoMeter {
                    Layout.fillWidth: true
                    leftDb: trackRow.meterPeaks.left ?? -70
                    rightDb: trackRow.meterPeaks.right ?? -70
                }
            }
            EchoParameterSlider {
                objectName: "trackGain"
                Layout.fillWidth: true
                Layout.preferredHeight: 22
                label: qsTr("Gain")
                labelWidth: 26
                valueWidth: 60
                from: -2400
                to: 1200
                stepSize: 50
                value: trackRow.track.gainCentibels
                valueText: (value / 100).toFixed(1) + " dB"
                showNeutralMarker: true
                neutralValue: 0
                property bool dragging: false
                onGestureStarted: dragging = true
                onGestureFinished: {
                    dragging = false;
                    trackRow.trackValueRequested(trackRow.trackIndex, "gainCentibels", Math.round(value));
                }
                onEdited: value => {
                    if (dragging)
                        trackRow.trackPreviewRequested(trackRow.trackIndex, "gainCentibels", Math.round(value));
                    else
                        trackRow.trackValueRequested(trackRow.trackIndex, "gainCentibels", Math.round(value));
                }
                accessibleName: qsTr("Track gain")
            }
            EchoParameterSlider {
                objectName: "trackPan"
                Layout.fillWidth: true
                Layout.preferredHeight: 22
                label: qsTr("Pan")
                labelWidth: 26
                valueWidth: 60
                from: -100
                to: 100
                stepSize: 1
                value: trackRow.track.panPercent
                valueText: value === 0 ? qsTr("Center") : (value < 0 ? "L " : "R ") + Math.abs(value)
                showNeutralMarker: true
                neutralValue: 0
                property bool dragging: false
                onGestureStarted: dragging = true
                onGestureFinished: {
                    dragging = false;
                    trackRow.trackValueRequested(trackRow.trackIndex, "panPercent", Math.round(value));
                }
                onEdited: value => {
                    if (dragging)
                        trackRow.trackPreviewRequested(trackRow.trackIndex, "panPercent", Math.round(value));
                    else
                        trackRow.trackValueRequested(trackRow.trackIndex, "panPercent", Math.round(value));
                }
                accessibleName: qsTr("Track pan")
            }
        }
        Menu {
            id: trackMenu
            MenuItem {
                text: qsTr("Reset track mix")
                onTriggered: {
                    trackRow.trackMixResetRequested(trackRow.trackIndex);
                }
            }
            MenuItem {
                text: qsTr("Delete track")
                enabled: trackRow.canDeleteTrack
                onTriggered: trackRow.trackDeleteRequested(trackRow.trackIndex)
            }
        }
    }
}
