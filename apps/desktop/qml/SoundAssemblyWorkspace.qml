//! Library-driven multi-asset arrangement workspace. It owns composition-time
//! editing while every clip remains pinned to one immutable asset revision.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import EchoDesktop
import "SoundAssemblyEditing.js" as Editing
import "SoundAssemblySelection.js" as Selection

Rectangle {
    id: workspace

    readonly property bool independentMode: backend.independentEditing === true
    property var document: ({})
    property var assemblies: []
    property var libraryAssets: []
    property string selectedClipId: ""
    property var selectedClipIds: []
    property string groupMoveId: ""
    property real groupMoveDelta: 0
    onSelectedClipIdChanged: {
        if (!selectedClipIds.includes(selectedClipId)) selectedClipIds = selectedClipId ? [selectedClipId] : [];
    }
    onDocumentChanged: { groupMoveId = ""; groupMoveDelta = 0; }
    readonly property var selectionBounds: Selection.bounds(tracks, selectedClipIds)
    readonly property int selectionCount: selectedClipIds.length
    property int selectedTrackIndex: -1
    property bool fitPending: false
    property real pixelsPerSecond: 90
    readonly property real trackHeaderWidth: 208
    property real playheadMillis: 0
    property var undoStack: []
    property var redoStack: []
    property bool dirty: false
    property string savedDocumentJson: ""
    property string errorText: ""
    property bool memorySavedNotice: false
    readonly property string noticeText: memorySavedNotice ? qsTr("This version is now in your memory library.") : ""
    property bool sourcesVisible: false
    property bool inspectorVisible: true
    property alias ducking: clipInspector.ducking
    property bool duckingVisible: false
    property bool automationEditing: false
    property bool snapping: true
    property bool followPlayback: true
    property real snapGuideMillis: -1
    property string previewDocumentJson: ""
    property string pendingPreviewJson: ""
    property bool playbackOwned: false
    property bool previewSelection: false
    property bool loopPreview: false
    readonly property real previewRangeStart: previewSelection && selectionBounds ? selectionBounds.start : 0
    readonly property real previewRangeEnd: previewSelection && selectionBounds ? selectionBounds.end : durationMillis
    readonly property string previewKey: authoredJson(document) + "|" + previewRangeStart + ":" + previewRangeEnd
    onPreviewKeyChanged: { if (playbackOwned) stopPlayback(); }
    property real previewStartMillis: 0
    readonly property bool previewCurrent: soundAssemblyController.hasPreview && previewDocumentJson === previewKey
    readonly property real scrollPosition: timelineFlick.contentX
    readonly property real laneViewportWidth: Math.max(1, timelineFlick.width - trackHeaderWidth)
    signal editClipRequested(var asset, var revision, string clipId)
    signal memoryOpened(string assemblyId)

    readonly property bool canUndo: undoStack.length > 0
    readonly property bool canRedo: redoStack.length > 0
    readonly property bool hasDocument: document && document.id !== undefined
    readonly property var tracks: hasDocument ? document.tracks : []
    readonly property real durationMillis: assemblyDuration(document)
    readonly property real timelineWidth: Math.max(laneViewportWidth, durationMillis * pixelsPerSecond / 1000 + 100)
    readonly property var selectedClip: clipById(selectedClipId)
    readonly property real tickStepSeconds: Editing.gridSeconds(pixelsPerSecond)

    color: Theme.window
    focus: true

    function clone(value: var): var {
        return JSON.parse(JSON.stringify(value));
    }

    function presentError(message: string): void {
        errorText = message;
        errorTimer.restart();
    }

    function authoredJson(value: var): string {
        if (!value || value.id === undefined)
            return "";
        return JSON.stringify({
            id: value.id,
            name: value.name,
            master: value.master,
            tracks: value.tracks
        });
    }

    function fadeCurveIndex(value: var): int {
        if (value === "smooth")
            return 1;
        if (value === "equal_power")
            return 2;
        return 0;
    }

    function fadeCurveValue(index: int): string {
        return index === 1 ? "smooth" : index === 2 ? "equal_power" : "linear";
    }

    function refreshAssemblies(): void {
        libraryAssets = backend.listAssets();
        assemblies = backend.listSoundAssemblies();
        if (!hasDocument && assemblies.length > 0)
            openAssembly(assemblies[0].assemblyId);
    }

    function loadRevision(revision: var): void {
        if (!revision || revision.error) {
            presentError(revision && revision.error ? revision.error : qsTr("The assembly could not be opened."));
            return;
        }
        stopPlayback();
        previewDocumentJson = "";
        pendingPreviewJson = "";
        document = clone(revision);
        selectedTrackIndex = document.tracks.length > 0 ? 0 : -1;
        selectedClipId = document.tracks.length > 0 && document.tracks[0].clips.length > 0 ? document.tracks[0].clips[0].id : "";
        selectedClipIds = selectedClipId ? [selectedClipId] : [];
        playheadMillis = 0;
        undoStack = [];
        redoStack = [];
        savedDocumentJson = authoredJson(document);
        dirty = false;
        errorText = "";
        refreshAssemblies();
        fitPending = true;
        Qt.callLater(fitProject);
    }

    function openAssembly(assemblyId: string): void {
        if (dirty && !saveRevision())
            return;
        soundAssemblyController.cancel();
        materialPlayer.stop();
        loadRevision(backend.soundAssembly(assemblyId));
    }

    function historySnapshot(): var {
        return {document: clone(document), ids: selectedClipIds.slice(), primary: selectedClipId};
    }

    function pushUndo(): void {
        const next = undoStack.slice();
        next.push(historySnapshot());
        if (next.length > 80)
            next.shift();
        undoStack = next;
        redoStack = [];
    }

    function mutate(callback: var): void {
        if (!hasDocument || soundAssemblyController.running)
            return;
        const next = clone(document);
        callback(next);
        if (authoredJson(next) === authoredJson(document))
            return;
        stopPlayback();
        pushUndo();
        document = next;
        dirty = authoredJson(next) !== savedDocumentJson;
        reconcileSelection();
    }

    function undo(): void {
        if (!canUndo)
            return;
        stopPlayback();
        const previous = undoStack.slice();
        const target = previous.pop();
        const future = redoStack.slice();
        future.push(historySnapshot());
        undoStack = previous;
        redoStack = future;
        document = target.document;
        selectedClipIds = target.ids;
        selectedClipId = target.primary;
        dirty = authoredJson(document) !== savedDocumentJson;
        reconcileSelection();
    }

    function redo(): void {
        if (!canRedo)
            return;
        stopPlayback();
        const future = redoStack.slice();
        const target = future.pop();
        const previous = undoStack.slice();
        previous.push(historySnapshot());
        undoStack = previous;
        redoStack = future;
        document = target.document;
        selectedClipIds = target.ids;
        selectedClipId = target.primary;
        dirty = authoredJson(document) !== savedDocumentJson;
        reconcileSelection();
    }

    function saveRevision(): var {
        if (!hasDocument)
            return null;
        const saved = backend.saveSoundAssembly(document);
        if (!saved || saved.error) {
            presentError(saved && saved.error ? saved.error : qsTr("The assembly version could not be saved."));
            return null;
        }
        document = clone(saved);
        savedDocumentJson = authoredJson(document);
        dirty = false;
        refreshAssemblies();
        reconcileSelection();
        return document;
    }

    function save(): void {
        saveRevision();
    }

    function preview(): void {
        materialPlayer.stop();
        const revision = dirty ? saveRevision() : document;
        if (revision) {
            pendingPreviewJson = previewKey;
            previewStartMillis = playheadMillis < previewRangeStart || playheadMillis >= previewRangeEnd ? previewRangeStart : playheadMillis;
            playbackOwned = true;
            soundAssemblyController.prepareRangePreview(revision, previewRangeStart, previewRangeEnd);
        }
    }

    function exportMix(): void {
        const revision = dirty ? saveRevision() : document;
        if (!revision)
            return;
        exportDialog.revision = clone(revision);
        exportDialog.open();
    }

    function keepMemory(): void {
        materialPlayer.stop();
        const revision = dirty ? saveRevision() : document;
        if (revision)
            soundAssemblyController.saveToMemory(revision);
    }

    function sourceAsset(clip: var): var {
        return clip ? libraryAssets.find(asset => asset.id === clip.assetId) || null : null;
    }
    function sourceName(clip: var): string {
        const asset = sourceAsset(clip);
        return asset ? SoundSemantics.sourceTitle(asset) : qsTr("Unavailable source");
    }
    function openClipEditor(): void {
        if (!selectedClip)
            return;
        const clipId = selectedClip.id;
        const revision = dirty ? saveRevision() : document;
        if (!revision)
            return;
        const source = revision.clipSources.find(source => source.clipId === clipId);
        const original = sourceAsset(clipById(clipId));
        if (!source || !original)
            return;
        const asset = clone(original);
        for (const key of Object.keys(source))
            asset[key] = source[key];
        asset.adjustmentRevision = source.adjustmentRevisionId;
        editClipRequested(asset, clone(revision), clipId);
    }
    function acceptClipRevision(revision: var): void {
        pushUndo();
        document = clone(revision);
        savedDocumentJson = authoredJson(document);
        dirty = false;
        refreshAssemblies();
        reconcileSelection();
    }

    function assemblyDuration(value: var): real {
        if (!value || !value.tracks)
            return 0;
        let maximum = 0;
        for (const track of value.tracks) {
            for (const clip of track.clips) {
                maximum = Math.max(maximum, clip.timelineStartMillis + clip.sourceEndMillis - clip.sourceStartMillis);
            }
        }
        return maximum;
    }

    function clipById(clipId: string): var {
        if (!hasDocument || clipId.length === 0)
            return null;
        for (let trackIndex = 0; trackIndex < document.tracks.length; ++trackIndex) {
            for (const clip of document.tracks[trackIndex].clips) {
                if (clip.id === clipId)
                    return clip;
            }
        }
        return null;
    }

    function clipLocation(clipId: string): var {
        if (!hasDocument)
            return null;
        for (let trackIndex = 0; trackIndex < document.tracks.length; ++trackIndex) {
            for (let clipIndex = 0; clipIndex < document.tracks[trackIndex].clips.length; ++clipIndex) {
                if (document.tracks[trackIndex].clips[clipIndex].id === clipId)
                    return ({
                            trackIndex: trackIndex,
                            clipIndex: clipIndex
                        });
            }
        }
        return null;
    }

    function reconcileSelection(): void {
        selectedClipIds = selectedClipIds.filter(id => clipLocation(id) !== null);
        if (!selectedClipIds.includes(selectedClipId)) selectedClipId = selectedClipIds[0] || "";
        let location = clipLocation(selectedClipId);
        if (location === null) {
            for (let index = 0; index < tracks.length; ++index) {
                if (tracks[index].clips.length) { selectedClipId = tracks[index].clips[0].id; location = {trackIndex: index}; break; }
            }
        }
        selectedTrackIndex = location ? location.trackIndex : tracks.length ? 0 : -1;
    }

    function applySelectionResult(result: var): bool {
        if (!hasDocument || soundAssemblyController.running) return false;
        if (result.error) {
            const messages = {capacity: qsTr("This edit would exceed the 256-clip limit."),
                duration: qsTr("This edit would exceed the four-hour timeline."),
                empty: qsTr("Keep at least one clip in the project."),
                split: qsTr("Place the playhead inside a selected clip before splitting."),
                track: qsTr("The selected clips cannot move beyond the first or last track.")};
            if (messages[result.error]) presentError(messages[result.error]);
            return false;
        }
        if (JSON.stringify(result.tracks) === JSON.stringify(tracks)) return false;
        const activeId = selectedClipId;
        mutate(next => next.tracks = result.tracks);
        selectedClipIds = result.ids || [];
        selectedClipId = selectedClipIds.includes(activeId) ? activeId : selectedClipIds[0] || "";
        reconcileSelection();
        return true;
    }

    function setTrackValue(trackIndex: int, key: string, value: var): void {
        mutate(next => next.tracks[trackIndex][key] = value);
    }

    function deleteTrack(trackIndex: int): void {
        if (trackIndex < 0 || trackIndex >= tracks.length || tracks.length <= 1 || totalClipCount() <= tracks[trackIndex].clips.length)
            return;
        mutate(next => next.tracks.splice(trackIndex, 1));
        reconcileSelection();
    }

    function setMasterValue(key: string, value: var): void {
        mutate(next => next.master[key] = value);
    }

    function setClipValue(key: string, value: var): void {
        const location = clipLocation(selectedClipId);
        if (location === null)
            return;
        mutate(next => next.tracks[location.trackIndex].clips[location.clipIndex][key] = value);
    }

    function moveClip(trackIndex: int, clipId: string, timelineStartMillis: real): void {
        const clip = clipById(clipId);
        if (!clip) return;
        if (!selectedClipIds.includes(clipId)) selectClip(trackIndex, clipId);
        applySelectionResult({tracks: Selection.move(tracks, selectedClipIds, timelineStartMillis - clip.timelineStartMillis), ids: selectedClipIds.slice()});
    }

    function sourceDurationFor(clip: var): real {
        const asset = sourceAsset(clip);
        const source = Editing.pinnedSource(clip, document.clipSources || [], libraryAssets);
        if (source === null)
            return clip.sourceEndMillis;
        const spans = Editing.sourceSpans(source, asset ? asset.durationMillis : clip.sourceEndMillis);
        return spans.length ? spans[spans.length - 1].end : clip.sourceEndMillis;
    }
    function setClipTiming(key: string, value: real): void {
        if (!selectedClip)
            return;
        if (key === "timelineStartMillis")
            moveClip(selectedTrackIndex, selectedClipId, value);
        else
            patchClip(selectedClipId, Editing.trim(selectedClip, key === "sourceStartMillis" ? "left" : "right", value - selectedClip[key], sourceDurationFor(selectedClip)));
    }
    function deleteSelectedClip(): void {
        applySelectionResult(Selection.remove(tracks, selectedClipIds));
    }

    function duplicateSelectedClip(): void {
        applySelectionResult(Selection.duplicate(tracks, selectedClipIds, () => backend.newAssemblyObjectId()));
    }

    function splitSelectedClip(): void {
        applySelectionResult(Selection.split(tracks, selectedClipIds, playheadMillis, () => backend.newAssemblyObjectId()));
    }

    function moveSelectedClipToTrack(delta: int): void {
        applySelectionResult(Selection.moveTracks(tracks, selectedClipIds, delta));
    }

    function addTrack(): void {
        if (!hasDocument || tracks.length >= 8)
            return;
        const id = backend.newAssemblyObjectId();
        mutate(next => next.tracks.push({
                id: id,
                name: qsTr("Track %1").arg(next.tracks.length + 1),
                gainCentibels: 0,
                panPercent: 0,
                muted: false,
                solo: false,
                clips: []
            }));
        selectedTrackIndex = tracks.length - 1;
    }

    function totalClipCount(): int {
        let count = 0;
        for (const track of tracks)
            count += track.clips.length;
        return count;
    }

    function linearAssetDuration(asset: var): real {
        if (Number(asset.adjustmentRevision || 0) <= 0 || !asset.editSegments || asset.editSegments.length === 0)
            return Number(asset.durationMillis || 0);
        let duration = 0;
        for (const segment of asset.editSegments) {
            if (Number(segment.state) !== 2)
                duration += Number(segment.sourceEndMillis) - Number(segment.sourceStartMillis);
            duration += Number(segment.gapAfterMillis || 0);
        }
        return duration;
    }

    function addLibraryAsset(asset: var, role: string): void {
        if (!asset || asset.assemblyId || totalClipCount() >= 256)
            return;
        if (!hasDocument) {
            const created = backend.createSoundAssembly((independentMode ? qsTr("Untitled project") : qsTr("New memory")), [asset.id], "sequence");
            loadRevision(created);
            if (hasDocument && role === "material")
                mutate(next => next.tracks[0].clips[0].sourceRole = role);
            return;
        }
        const targetTrack = selectedTrackIndex >= 0 ? selectedTrackIndex : 0;
        const duration = linearAssetDuration(asset);
        if (duration <= 0 || playheadMillis + duration > 14400000) {
            presentError(qsTr("This sound has no usable duration."));
            return;
        }
        const clipId = backend.newAssemblyObjectId();
        mutate(next => next.tracks[targetTrack].clips.push({
                id: clipId,
                assetId: asset.id,
                sourceRole: role || "memory",
                adjustmentRevisionId: Number(asset.adjustmentRevision || 0),
                sourceStartMillis: 0,
                sourceEndMillis: duration,
                timelineStartMillis: Math.round(playheadMillis),
                gainCentibels: 0,
                panPercent: 0,
                fadeInMillis: 0,
                fadeOutMillis: 0,
                fadeInCurve: "linear",
                fadeOutCurve: "linear",
                muted: false
            }));
        selectedClipId = clipId;
        selectedTrackIndex = targetTrack;
    }

    function formatTime(millis: real): string {
        const totalSeconds = Math.max(0, Math.floor(millis / 1000));
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor(totalSeconds % 3600 / 60);
        const seconds = totalSeconds % 60;
        return hours > 0 ? String(hours).padStart(2, "0") + ":" + String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0") : String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0");
    }

    function selectClip(trackIndex: int, clipId: string, modifiers: int, preserve: bool): void {
        if (modifiers & (Qt.ControlModifier | Qt.MetaModifier | Qt.ShiftModifier)) {
            selectedClipIds = selectedClipIds.includes(clipId) ? selectedClipIds.filter(id => id !== clipId) : selectedClipIds.concat([clipId]);
            selectedClipId = selectedClipIds.includes(clipId) ? clipId : selectedClipIds[selectedClipIds.length - 1] || "";
        } else {
            if (!preserve || !selectedClipIds.includes(clipId)) selectedClipIds = [clipId];
            selectedClipId = clipId;
        }
        const location = clipLocation(selectedClipId);
        selectedTrackIndex = location ? location.trackIndex : trackIndex;
        forceActiveFocus();
    }
    function selectAllClips(): void {
        selectedClipIds = tracks.reduce((all, track) => all.concat(track.clips.map(clip => clip.id)), []);
        selectedClipId = selectedClipIds[0] || "";
        reconcileSelection();
        forceActiveFocus();
    }

    function patchClip(clipId: string, patch: var, kind: string): void {
        const location = clipLocation(clipId);
        if (!location) return;
        if (kind === "move") {
            moveClip(location.trackIndex, clipId, patch.timelineStartMillis);
            return;
        }
        mutate(next => next.tracks[location.trackIndex].clips[location.clipIndex] = clone(patch));
    }

    function snapPosition(position: real, length: real, excludedId: string, bypass: bool): var {
        const moving = length > 0 && selectedClipIds.includes(excludedId);
        const excluded = moving ? selectedClipIds : [excludedId];
        const points = [0, playheadMillis];
        for (const track of tracks) for (const clip of track.clips) {
            if (!excluded.includes(clip.id)) points.push(clip.timelineStartMillis, Editing.end(clip));
        }
        const clip = clipById(excludedId);
        const limit = moving ? Selection.moveLimits(tracks, selectedClipIds) : null;
        const low = limit ? clip.timelineStartMillis + limit.minimum : 0;
        const high = limit ? clip.timelineStartMillis + limit.maximum : 14400000 - length;
        const result = Editing.snap(Editing.clamp(position, low, high), length, points, tickStepSeconds * 1000, 8 * 1000 / pixelsPerSecond, snapping && !bypass, high);
        if (result.position < low) return {position: low, guide: -1};
        return result;
    }

    function crossfadeCandidate(): var {
        if (!selectedClip || selectedTrackIndex < 0)
            return null;
        const candidates = tracks[selectedTrackIndex].clips.filter(clip => clip.id !== selectedClipId).map(clip => Editing.crossfade(selectedClip, clip)).filter(value => value);
        candidates.sort((a, b) => a.duration - b.duration);
        return candidates.length ? candidates[0] : null;
    }
    function crossfadeSelected(): void {
        const candidate = crossfadeCandidate();
        if (!candidate)
            return;
        mutate(next => {
            for (const clip of next.tracks[selectedTrackIndex].clips) {
                if (clip.id === candidate.first) {
                    clip.fadeOutMillis = candidate.duration;
                    clip.fadeOutCurve = "equal_power";
                }
                if (clip.id === candidate.second) {
                    clip.fadeInMillis = candidate.duration;
                    clip.fadeInCurve = "equal_power";
                }
            }
        });
    }
    function rippleDelete(allTracks: bool): void {
        const result = Selection.ripple(tracks, selectedClipIds, !!allTracks, () => backend.newAssemblyObjectId());
        if (applySelectionResult(result)) seekTo(result.position);
    }

    function stopPlayback(): void {
        const owned = playbackOwned;
        playbackOwned = false;
        if (owned) player.stop();
    }
    function togglePlayback(): void {
        if (soundAssemblyController.running) {
            soundAssemblyController.cancel();
            pendingPreviewJson = "";
            return;
        }
        if (!hasDocument)
            return;
        materialPlayer.stop();
        if (previewCurrent) {
            if (playbackOwned && player.active)
                player.togglePause();
            else {
                playbackOwned = true;
                soundAssemblyController.playPreview(Math.round(playheadMillis < previewRangeStart || playheadMillis >= previewRangeEnd ? 0 : playheadMillis - previewRangeStart));
            }
        } else
            preview();
    }
    function seekTo(position: real): void {
        playheadMillis = Editing.clamp(Math.round(position), 0, durationMillis);
        if (playbackOwned && previewCurrent && player.active)
            player.seek(Math.round(Editing.clamp(playheadMillis - previewRangeStart, 0, previewRangeEnd - previewRangeStart)));
    }
    function fitProject(): void {
        if (!hasDocument) return;
        if (!visible || laneViewportWidth <= 80) { fitPending = true; return; }
        fitPending = false;
        pixelsPerSecond = Editing.clamp((laneViewportWidth - 80) * 1000 / Math.max(1000, durationMillis), 0.02, 800);
        timelineFlick.contentX = 0;
    }
    function fitSelection(): void {
        if (!selectionBounds)
            return;
        pixelsPerSecond = Editing.clamp((laneViewportWidth - 80) * 1000 / Math.max(10, selectionBounds.end - selectionBounds.start), 0.02, 800);
        Qt.callLater(() => { if (selectionBounds) timelineFlick.contentX = Math.max(0, selectionBounds.start * pixelsPerSecond / 1000 - 40); });
    }
    function zoomBy(factor: real): void {
        const anchor = Math.max(0, (playheadMillis * pixelsPerSecond / 1000 - timelineFlick.contentX));
        pixelsPerSecond = Editing.clamp(pixelsPerSecond * factor, 0.02, 800);
        timelineFlick.contentX = Editing.clamp(playheadMillis * pixelsPerSecond / 1000 - anchor, 0, Math.max(0, timelineWidth - laneViewportWidth));
    }
    function refreshWaveforms(): void {
        const ids = [];
        for (const track of tracks)
            for (const clip of track.clips)
                if (ids.indexOf(clip.assetId) < 0)
                    ids.push(clip.assetId);
        assemblyWaveforms.setSources(ids);
    }
    onTracksChanged: refreshWaveforms()
    onLaneViewportWidthChanged: if (fitPending && visible) Qt.callLater(fitProject)
    onVisibleChanged: {
        if (visible && fitPending) Qt.callLater(fitProject);
        if (!visible) {
            if (pendingPreviewJson) {
                pendingPreviewJson = "";
                soundAssemblyController.cancel();
            }
            stopPlayback();
        }
    }
    Keys.onPressed: event => {
        if (!hasDocument || soundAssemblyController.running)
            return;
        const command = event.modifiers & (Qt.ControlModifier | Qt.MetaModifier);
        if (event.key === Qt.Key_Space)
            togglePlayback();
        else if (event.key === Qt.Key_S && !command)
            splitSelectedClip();
        else if (event.key === Qt.Key_A && command)
            selectAllClips();
        else if (event.key === Qt.Key_D && command)
            duplicateSelectedClip();
        else if (event.key === Qt.Key_Z && command) {
            if (event.modifiers & Qt.ShiftModifier)
                redo();
            else
                undo();
        } else if (event.key === Qt.Key_Delete || event.key === Qt.Key_Backspace) {
            if (event.modifiers & Qt.ShiftModifier)
                rippleDelete();
            else
                deleteSelectedClip();
        } else if (event.key === Qt.Key_Left || event.key === Qt.Key_Right) {
            const step = (event.modifiers & Qt.ShiftModifier ? 100 : 10) * (event.key === Qt.Key_Left ? -1 : 1);
            if (selectedClip)
                moveClip(selectedTrackIndex, selectedClipId, selectedClip.timelineStartMillis + step);
        } else if (event.key === Qt.Key_Home)
            seekTo(0);
        else if (event.key === Qt.Key_F)
            fitProject();
        else if (event.key === Qt.Key_Plus || event.key === Qt.Key_Equal)
            zoomBy(1.3);
        else if (event.key === Qt.Key_Minus)
            zoomBy(1 / 1.3);
        else
            return;
        event.accepted = true;
    }

    Menu {
        id: clipMenu
        MenuItem {
            text: qsTr("Edit this clip’s sound")
            onTriggered: workspace.openClipEditor()
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Split at playhead")
            onTriggered: workspace.splitSelectedClip()
        }
        MenuItem {
            text: qsTr("Duplicate after selection")
            onTriggered: workspace.duplicateSelectedClip()
        }
        MenuItem {
            text: qsTr("Crossfade overlap")
            enabled: workspace.crossfadeCandidate() !== null
            onTriggered: workspace.crossfadeSelected()
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Delete selected clips")
            enabled: workspace.totalClipCount() > 1
            onTriggered: workspace.deleteSelectedClip()
        }
        MenuItem {
            text: qsTr("Ripple delete time ranges on selected tracks")
            enabled: workspace.totalClipCount() > 1
            onTriggered: workspace.rippleDelete(false)
        }
        MenuItem {
            text: qsTr("Ripple delete time ranges on all tracks")
            enabled: workspace.selectionCount > 0 && workspace.totalClipCount() > workspace.selectionCount
            onTriggered: workspace.rippleDelete(true)
        }
        MenuSeparator {}
        MenuItem { text: qsTr("Select all clips"); onTriggered: workspace.selectAllClips() }
    }
    Connections {
        target: player
        function onStateChanged(): void {
            if (workspace.playbackOwned && player.errorText) {
                workspace.playbackOwned=false;
                workspace.presentError(player.errorText);
                return;
            }
            if (!player.active && workspace.visible && workspace.playbackOwned && workspace.previewCurrent && workspace.loopPreview)
                Qt.callLater(() => { if (workspace.playbackOwned && workspace.previewCurrent && workspace.loopPreview) soundAssemblyController.playPreview(); });
        }
        function onPositionChanged(): void {
            if (!workspace.visible || !workspace.playbackOwned || !workspace.previewCurrent)
                return;
            workspace.playheadMillis = workspace.previewRangeStart + player.position;
            const x = workspace.playheadMillis * workspace.pixelsPerSecond / 1000;
            if (workspace.followPlayback && player.playing && (x < timelineFlick.contentX || x > timelineFlick.contentX + workspace.laneViewportWidth - 50))
                timelineFlick.contentX = Math.max(0, x - 50);
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            visible: workspace.sourcesVisible
            Layout.preferredWidth: 282
            Layout.minimumWidth: 282
            Layout.maximumWidth: 282
            Layout.fillHeight: true
            spacing: 0
            EchoComboBox {
                Layout.fillWidth: true
                Layout.margins: 12
                model: workspace.assemblies
                textRole: "name"
                selectionIndex: workspace.assemblies.findIndex(value => workspace.hasDocument && value.assemblyId === workspace.document.id)
                displayText: workspace.hasDocument ? workspace.document.name : qsTr("Choose a project")
                onActivated: workspace.openAssembly(workspace.assemblies[currentIndex].assemblyId)
            }
            SoundSourceBrowser {
                id: sourceBrowser
                Layout.fillWidth: true
                Layout.fillHeight: true
                editorMode: true
                assemblyId: workspace.hasDocument ? workspace.document.id : ""
                projectDocument: workspace.document
                onAddRequested: (asset, role) => workspace.addLibraryAsset(asset, role)
                onOpenAssemblyRequested: id => workspace.openAssembly(id)
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 56
                color: Theme.chrome
                border.width: 1
                border.color: Theme.border

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    spacing: 8

                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/folder.svg"
                        toolTipText: qsTr("Show sources and projects")
                        selected: workspace.sourcesVisible
                        onClicked: workspace.sourcesVisible = !workspace.sourcesVisible
                    }
                    EchoTextField {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 100
                        enabled: workspace.hasDocument
                        text: workspace.hasDocument ? workspace.document.name : ""
                        placeholderText: qsTr("Assembly name")
                        onEditingFinished: {
                            if (workspace.hasDocument && text.trim().length > 0 && text !== workspace.document.name)
                                workspace.mutate(next => next.name = text.trim());
                        }
                    }

                    EchoButton {
                        text: qsTr("Add sound")
                        ghost: true
                        enabled: workspace.hasDocument && workspace.totalClipCount() < 256
                        onClicked: {
                            workspace.sourcesVisible = !workspace.sourcesVisible;
                            if (workspace.sourcesVisible)
                                sourceBrowser.sourceTab = workspace.independentMode ? 0 : 2;
                        }
                    }

                    Text {
                        visible: soundAssemblyController.running
                        text: qsTr("Preparing %1%").arg(Math.round(soundAssemblyController.progress * 100))
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }

                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/fit-selection.svg"
                                toolTipText: qsTr("Preview selection range")
                        selected: workspace.previewSelection
                        enabled: workspace.selectedClip !== null && !soundAssemblyController.running
                        onClicked: workspace.previewSelection = !workspace.previewSelection
                    }
                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/loop.svg"
                        toolTipText: qsTr("Loop preview")
                        selected: workspace.loopPreview
                        onClicked: workspace.loopPreview = !workspace.loopPreview
                    }
                    EchoButton {
                        objectName: "assemblyPreviewButton"
                        text: soundAssemblyController.running ? qsTr("Cancel") : workspace.playbackOwned && player.playing ? qsTr("Pause") : qsTr("Preview")
                        enabled: workspace.hasDocument
                        onClicked: workspace.togglePlayback()
                    }
                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/tune.svg"
                        toolTipText: qsTr("Show clip inspector")
                        selected: workspace.inspectorVisible
                        onClicked: workspace.inspectorVisible = !workspace.inspectorVisible
                    }

                    EchoButton {
                        visible: !workspace.independentMode
                        text: qsTr("Keep in memories")
                        enabled: workspace.hasDocument && !soundAssemblyController.running
                        onClicked: workspace.keepMemory()
                    }

                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/more-horizontal.svg"
                        toolTipText: qsTr("Assembly actions")
                        enabled: workspace.hasDocument
                        onClicked: assemblyMenu.popup()
                    }
                }

                Menu {
                    id: assemblyMenu
                    MenuItem {
                        text: qsTr("Mixdown…")
                        enabled: workspace.hasDocument && !soundAssemblyController.running
                        onTriggered: workspace.exportMix()
                    }
                    MenuItem {
                        text: qsTr("Archive assembly")
                        onTriggered: {
                            if (backend.archiveSoundAssembly(workspace.document.id)) {
                                workspace.document = ({});
                                workspace.savedDocumentJson = "";
                                workspace.dirty = false;
                                workspace.refreshAssemblies();
                            } else {
                                workspace.presentError(qsTr("The assembly could not be archived."));
                            }
                        }
                    }
                }
            }

            ProgressBar {
                Layout.fillWidth: true
                Layout.preferredHeight: soundAssemblyController.running ? 3 : 0
                visible: soundAssemblyController.running
                from: 0
                to: 1
                value: soundAssemblyController.progress
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 0

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 38
                        color: Theme.panelRaised
                        RowLayout {
                            x: 10
                            height: parent.height
                            width: workspace.trackHeaderWidth - 20
                            spacing: 4
                            Text {
                                text: qsTr("TRACKS")
                                font.pixelSize: 9
                                font.letterSpacing: 1
                                color: Theme.textMuted
                                Layout.fillWidth: true
                            }
                            EchoIconButton {
                                source: "qrc:/EchoDesktop/icons/plus.svg"
                                toolTipText: qsTr("Add track")
                                enabled: workspace.hasDocument && workspace.tracks.length < 8
                                onClicked: workspace.addTrack()
                            }
                        }
                        Item {
                            x: workspace.trackHeaderWidth
                            width: parent.width - x
                            height: parent.height
                            clip: true
                            Repeater {
                                model: Math.ceil(parent.width / (workspace.tickStepSeconds * workspace.pixelsPerSecond)) + 2
                                delegate: Item {
                                    required property int index
                                    readonly property int tick: Math.floor(timelineFlick.contentX / (workspace.tickStepSeconds * workspace.pixelsPerSecond)) + index
                                    x: tick * workspace.tickStepSeconds * workspace.pixelsPerSecond - timelineFlick.contentX
                                    width: 1
                                    height: parent.height
                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        width: 1
                                        height: 9
                                        color: Theme.borderStrong
                                    }
                                    Text {
                                        x: 5
                                        y: 6
                                        text: workspace.tickStepSeconds < 1 ? (tick * workspace.tickStepSeconds).toFixed(1) + " s" : workspace.formatTime(tick * workspace.tickStepSeconds * 1000)
                                        color: Theme.textMuted
                                        font.pixelSize: 9
                                        font.family: "Menlo"
                                    }
                                }
                            }
                            Rectangle {
                                x: workspace.playheadMillis * workspace.pixelsPerSecond / 1000 - timelineFlick.contentX - 4
                                anchors.bottom: parent.bottom
                                width: 8
                                height: 8
                                rotation: 45
                                color: Theme.accent
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onPressed: mouse => {
                                    workspace.forceActiveFocus();
                                    workspace.seekTo((mouse.x + timelineFlick.contentX) * 1000 / workspace.pixelsPerSecond);
                                }
                                onPositionChanged: mouse => {
                                    if (pressed)
                                        workspace.seekTo((mouse.x + timelineFlick.contentX) * 1000 / workspace.pixelsPerSecond);
                                }
                            }
                        }
                    }
                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Flickable {
                            id: timelineFlick
                            objectName: "assemblyTimeline"
                            anchors.fill: parent
                            contentWidth: workspace.trackHeaderWidth + workspace.timelineWidth
                            contentHeight: Math.max(height, trackColumn.height + 30)
                            clip: true
                            boundsBehavior: Flickable.StopAtBounds
                            acceptedButtons: Qt.MiddleButton
                            Column {
                                id: trackColumn
                                width: timelineFlick.contentWidth
                                spacing: 1
                                enabled: !soundAssemblyController.running
                                Repeater {
                                    model: workspace.tracks
                                    delegate: SoundAssemblyTrack {
                                        required property var modelData
                                        required property int index
                                        width: trackColumn.width
                                        automationEditing: workspace.automationEditing
                                        sourceAssets: workspace.libraryAssets
                                        clipSources: workspace.document.clipSources || []
                                        waveforms: assemblyWaveforms.waveforms
                                        snapPosition: workspace.snapPosition
                                        horizontalOffset: timelineFlick.contentX
                                        viewportWidth: timelineFlick.width
                                        anySolo: workspace.tracks.some(track => track.solo)
                                        track: modelData
                                        trackIndex: index
                                        pixelsPerSecond: workspace.pixelsPerSecond
                                        playheadMillis: workspace.playheadMillis
                                        selectedClipId: workspace.selectedClipId
                                        selectedClipIds: workspace.selectedClipIds
                                        groupMoveId: workspace.groupMoveId
                                        groupMoveDelta: workspace.groupMoveDelta
                                        onMovePreviewRequested: (clipId, delta) => { workspace.groupMoveId = clipId; workspace.groupMoveDelta = delta; }
                                        canDeleteTrack: workspace.tracks.length > 1 && workspace.totalClipCount() > modelData.clips.length
                                        headerWidth: workspace.trackHeaderWidth
                                        timelineWidth: workspace.timelineWidth
                                        onSourceDropped: (asset, targetTrack, positionMillis) => {
                                            workspace.selectedTrackIndex = targetTrack;
                                            workspace.playheadMillis = positionMillis;
                                            workspace.addLibraryAsset(asset, sourceBrowser.roleFor(asset));
                                        }
                                        onClipEditRequested: (trackIndex, clipId) => {
                                            workspace.selectClip(trackIndex, clipId);
                                            workspace.openClipEditor();
                                        }
                                        onTrackValueRequested: workspace.setTrackValue(trackIndex, key, value)
                                        onTrackMixResetRequested: trackIndex => workspace.mutate(next => {
                                                next.tracks[trackIndex].gainCentibels = 0;
                                                next.tracks[trackIndex].panPercent = 0;
                                            })
                                        onTrackDeleteRequested: workspace.deleteTrack(trackIndex)
                                        onClipSelected: (trackIndex, clipId, modifiers, preserve) => workspace.selectClip(trackIndex, clipId, modifiers, preserve)
                                        onClipPatchRequested: (clipId, patch, kind) => workspace.patchClip(clipId, patch, kind)
                                        onContextRequested: clipMenu.popup()
                                        onGuideChanged: position => workspace.snapGuideMillis = position
                                        onSeekRequested: position => {
                                            workspace.forceActiveFocus();
                                            workspace.seekTo(position);
                                        }
                                    }
                                }
                            }
                            ScrollBar.horizontal: ScrollBar {
                                policy: ScrollBar.AsNeeded
                            }
                            ScrollBar.vertical: ScrollBar {
                                policy: ScrollBar.AsNeeded
                            }
                        }
                        Item {
                            x: workspace.trackHeaderWidth
                            width: parent.width - x
                            height: parent.height
                            clip: true
                            Repeater {
                                model: Math.ceil(parent.width / (workspace.tickStepSeconds * workspace.pixelsPerSecond)) + 2
                                delegate: Rectangle {
                                    required property int index
                                    readonly property int tick: Math.floor(timelineFlick.contentX / (workspace.tickStepSeconds * workspace.pixelsPerSecond)) + index
                                    x: tick * workspace.tickStepSeconds * workspace.pixelsPerSecond - timelineFlick.contentX
                                    width: 1
                                    height: parent.height
                                    color: Qt.alpha(Theme.borderStrong, 0.18)
                                }
                            }
                            Rectangle {
                                x: workspace.playheadMillis * workspace.pixelsPerSecond / 1000 - timelineFlick.contentX
                                width: 1
                                height: parent.height
                                color: Theme.accent
                            }
                            Rectangle {
                                visible: workspace.snapGuideMillis >= 0
                                x: workspace.snapGuideMillis * workspace.pixelsPerSecond / 1000 - timelineFlick.contentX
                                width: 1
                                height: parent.height
                                color: Theme.warningText
                            }
                        }
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 46
                        color: Theme.chrome
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            spacing: 6
                            Text {
                                text: workspace.formatTime(workspace.playheadMillis) + "." + String(Math.floor(workspace.playheadMillis % 1000)).padStart(3, "0")
                                color: Theme.textPrimary
                                font.family: "Menlo"
                                font.pixelSize: 12
                                Layout.preferredWidth: 104
                            }
                            EchoIconButton {
                                source: "qrc:/EchoDesktop/icons/stop.svg"
                                toolTipText: qsTr("Stop and return to start")
                                onClicked: {
                                    workspace.stopPlayback();
                                    workspace.seekTo(0);
                                }
                            }
                            EchoButton {
                                text: qsTr("Snap")
                                ghost: true
                                checkable: true
                                checked: workspace.snapping
                                selected: checked
                                onClicked: workspace.snapping = checked
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Snap to clips, playhead and grid · Hold Shift to bypass")
                            }
                            EchoIconButton {
                                source: "qrc:/EchoDesktop/icons/follow-playhead.svg"
                                toolTipText: qsTr("Follow playback")
                                selected: workspace.followPlayback
                                onClicked: workspace.followPlayback = !workspace.followPlayback
                            }
                            Item {
                                Layout.fillWidth: true
                            }
                            Text {
                                visible: workspace.selectionCount > 1
                                text: qsTr("%1 clips selected").arg(workspace.selectionCount)
                                font.pixelSize: Theme.fontMeta
                                color: Theme.textSecondary
                            }
                            EchoIconButton {
                                source: "qrc:/EchoDesktop/icons/fit-all.svg"
                                toolTipText: qsTr("Fit project (F)")
                                onClicked: workspace.fitProject()
                            }
                            EchoIconButton {
                                source: "qrc:/EchoDesktop/icons/fit-selection.svg"
                                toolTipText: qsTr("Fit selection")
                                enabled: workspace.selectedClip !== null
                                onClicked: workspace.fitSelection()
                            }
                            EchoButton {
                                text: "−"
                                ghost: true
                                implicitWidth: 25
                                onClicked: workspace.zoomBy(1 / 1.3)
                                Accessible.name: qsTr("Zoom out")
                            }
                            EchoButton {
                                text: "+"
                                ghost: true
                                implicitWidth: 25
                                onClicked: workspace.zoomBy(1.3)
                                Accessible.name: qsTr("Zoom in")
                            }
                        }
                    }
                }

                SoundAssemblyInspector {
                    id: clipInspector
                    visible: workspace.inspectorVisible
                    Layout.preferredWidth: 280
                    Layout.minimumWidth: 280
                    Layout.maximumWidth: 280
                    Layout.fillHeight: true
                    workspace: workspace
                    renderController: soundAssemblyController
                    waveforms: assemblyWaveforms.waveforms
                }
            }
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 440)
        visible: !workspace.hasDocument
        spacing: 12
        z: 20

        EchoIcon {
            Layout.alignment: Qt.AlignHCenter
            source: "qrc:/EchoDesktop/icons/assembly.svg"
            size: 34
            color: Theme.textDisabled
        }
        Text {
            Layout.fillWidth: true
            text: qsTr("Build a sound sequence or layered scene")
            color: Theme.textPrimary
            font.pixelSize: 18
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
        }
        Text {
            Layout.fillWidth: true
            text: qsTr("Select sounds in Audio Space, then choose Sequence or Layer. Existing assemblies remain available in the sidebar.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    FileDialog {
        id: exportDialog
        property var revision: null
        title: qsTr("Export assembly mixdown")
        fileMode: FileDialog.SaveFile
        nameFilters: [qsTr("WAV audio (*.wav)")]
        defaultSuffix: "wav"
        onAccepted: soundAssemblyController.exportAssembly(revision, selectedFile)
    }

    Popup {
        id: errorPopup
        parent: Overlay.overlay
        x: Math.round((parent.width - width) / 2)
        y: 18
        width: Math.min(620, errorLabel.implicitWidth + 36)
        height: errorLabel.implicitHeight + 24
        visible: workspace.errorText.length > 0 || soundAssemblyController.errorText.length > 0 || workspace.noticeText.length > 0
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            radius: Theme.controlRadius
            color: workspace.errorText || soundAssemblyController.errorText ? Theme.warningSurface : Theme.accentSurface
            border.width: 1
            border.color: workspace.errorText || soundAssemblyController.errorText ? Theme.warningText : Theme.accentBorder
        }

        contentItem: Text {
            id: errorLabel
            text: workspace.errorText || soundAssemblyController.errorText || workspace.noticeText
            color: workspace.errorText || soundAssemblyController.errorText ? Theme.warningText : Theme.accentSelectionText
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    Connections {
        target: backend
        function onAssetsChanged(): void {
            workspace.libraryAssets = backend.listAssets();
        }
    }
    Connections {
        target: soundAssemblyController
        function onStateChanged(): void {
            if (!soundAssemblyController.running && workspace.pendingPreviewJson) {
                if (soundAssemblyController.hasPreview && !soundAssemblyController.errorText) {
                    workspace.previewDocumentJson = workspace.pendingPreviewJson;
                    if (workspace.previewCurrent && workspace.visible) {
                        workspace.playbackOwned = true;
                        soundAssemblyController.playPreview(Math.round(workspace.previewStartMillis - workspace.previewRangeStart));
                    } else
                        workspace.stopPlayback();
                }
                workspace.pendingPreviewJson = "";
            }
        }
        function onMemorySaved(assemblyId): void {
            workspace.memorySavedNotice = true;
            errorTimer.restart();
        }
    }
    Timer {
        id: errorTimer
        interval: 5000
        onTriggered: {
            workspace.errorText = "";
            workspace.memorySavedNotice = false;
        }
    }

    Connections {
        target: backend
        function onSoundAssembliesChanged(): void {
            workspace.refreshAssemblies();
        }
    }

    Component.onCompleted: refreshAssemblies()
}
