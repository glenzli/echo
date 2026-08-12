//! Local listening-continuity owner. It checkpoints only meaningful audition
//! sessions and keeps playback lifecycle concerns out of the Library views.

import QtQuick

Item {
    id: tracker

    required property var asset
    required property string loadedPath
    required property int playbackStartMillis
    required property int playbackEndMillis
    property bool active: true
    property var catalogBackend: backend
    property var playbackController: player
    property int resumePositionMillis: asset ? Number(asset.resumePositionMillis || 0) : 0

    property string trackedAssetId: ""
    property string trackedPath: ""
    property int trackedStartMillis: 0
    property int trackedEndMillis: 0
    property int listenedMillis: 0
    property int sinceCheckpointMillis: 0
    property int lastObservedPositionMillis: 0
    property int lastCheckpointPositionMillis: -1

    readonly property bool ownsPlayback: active && asset && asset.id && loadedPath.length > 0 && loadedPath === asset.path && playbackController.active

    signal checkpointRecorded(string assetId, double lastListenedAtMillis, double resumePositionMillis)

    width: 0
    height: 0
    visible: false

    function beginIfNeeded(): void {
        if (!ownsPlayback)
            return;
        if (trackedAssetId === asset.id && trackedPath === loadedPath)
            return;
        finishTracking();
        trackedAssetId = asset.id;
        trackedPath = loadedPath;
        trackedStartMillis = playbackStartMillis;
        trackedEndMillis = playbackEndMillis;
        listenedMillis = 0;
        sinceCheckpointMillis = 0;
        lastObservedPositionMillis = Number(playbackController.position || playbackStartMillis);
        lastCheckpointPositionMillis = -1;
    }

    function recordCheckpoint(): void {
        if (trackedAssetId.length === 0 || listenedMillis < 3000)
            return;
        const position = Math.max(trackedStartMillis, Math.min(lastObservedPositionMillis, trackedEndMillis));
        if (lastCheckpointPositionMillis >= 0 && Math.abs(position - lastCheckpointPositionMillis) < 1000)
            return;
        const state = catalogBackend.recordListeningProgress(trackedAssetId, position, trackedStartMillis, trackedEndMillis);
        if (!state || Number(state.lastListenedAtMillis || 0) <= 0)
            return;
        lastCheckpointPositionMillis = position;
        sinceCheckpointMillis = 0;
        resumePositionMillis = Number(state.resumePositionMillis || 0);
        checkpointRecorded(trackedAssetId, Number(state.lastListenedAtMillis), resumePositionMillis);
    }

    function finishTracking(): void {
        recordCheckpoint();
        trackedAssetId = "";
        trackedPath = "";
        trackedStartMillis = 0;
        trackedEndMillis = 0;
        listenedMillis = 0;
        sinceCheckpointMillis = 0;
        lastObservedPositionMillis = 0;
        lastCheckpointPositionMillis = -1;
    }

    onAssetChanged: {
        if (trackedAssetId.length === 0)
            resumePositionMillis = asset ? Number(asset.resumePositionMillis || 0) : 0;
    }
    onLoadedPathChanged: {
        if (trackedPath.length > 0 && trackedPath !== loadedPath)
            finishTracking();
        beginIfNeeded();
    }
    onActiveChanged: {
        if (!active)
            finishTracking();
        else
            beginIfNeeded();
    }

    Timer {
        interval: 1000
        repeat: true
        running: tracker.ownsPlayback && tracker.playbackController.playing
        onTriggered: {
            tracker.beginIfNeeded();
            tracker.listenedMillis += interval;
            tracker.sinceCheckpointMillis += interval;
            tracker.lastObservedPositionMillis = Number(tracker.playbackController.position);
            if (tracker.sinceCheckpointMillis >= 5000)
                tracker.recordCheckpoint();
        }
    }

    Connections {
        target: tracker.playbackController

        function onPositionChanged(): void {
            if (tracker.trackedAssetId.length > 0)
                tracker.lastObservedPositionMillis = Number(tracker.playbackController.position);
        }

        function onStateChanged(): void {
            if (!tracker.playbackController.playing)
                tracker.recordCheckpoint();
            if (!tracker.playbackController.active)
                tracker.finishTracking();
            else
                tracker.beginIfNeeded();
        }
    }
}
