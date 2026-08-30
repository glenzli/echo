//! Library-driven multi-asset arrangement workspace. It owns composition-time
//! editing while every clip remains pinned to one immutable asset revision.

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: workspace

    property var document: ({})
    property var assemblies: []
    property var libraryAssets: []
    property string selectedClipId: ""
    property int selectedTrackIndex: -1
    property real pixelsPerSecond: 90
    readonly property real trackHeaderWidth: 220
    property real playheadMillis: 0
    property var undoStack: []
    property var redoStack: []
    property bool dirty: false
    property string savedDocumentJson: ""
    property string errorText: ""

    readonly property bool canUndo: undoStack.length > 0
    readonly property bool canRedo: redoStack.length > 0
    readonly property bool hasDocument: document && document.id !== undefined
    readonly property var tracks: hasDocument ? document.tracks : []
    readonly property real durationMillis: assemblyDuration(document)
    readonly property real timelineWidth: Math.max(900, durationMillis * pixelsPerSecond / 1000 + 360)
    readonly property var selectedClip: clipById(selectedClipId)
    readonly property real tickStepSeconds: durationMillis <= 300000 ? 1 : durationMillis <= 3600000 ? 10 : 60
    readonly property int tickCount: Math.ceil(durationMillis / 1000 / tickStepSeconds) + 2

    color: Theme.window

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
        assemblies = backend.listSoundAssemblies();
        if (!hasDocument && assemblies.length > 0)
            openAssembly(assemblies[0].assemblyId);
    }

    function loadRevision(revision: var): void {
        if (!revision || revision.error) {
            presentError(revision && revision.error ? revision.error : qsTr("The assembly could not be opened."));
            return;
        }
        document = clone(revision);
        selectedTrackIndex = document.tracks.length > 0 ? 0 : -1;
        selectedClipId = document.tracks.length > 0 && document.tracks[0].clips.length > 0
            ? document.tracks[0].clips[0].id : "";
        playheadMillis = 0;
        undoStack = [];
        redoStack = [];
        savedDocumentJson = authoredJson(document);
        dirty = false;
        errorText = "";
        refreshAssemblies();
    }

    function openAssembly(assemblyId: string): void {
        loadRevision(backend.soundAssembly(assemblyId));
    }

    function pushUndo(): void {
        const next = undoStack.slice();
        next.push(clone(document));
        if (next.length > 80)
            next.shift();
        undoStack = next;
        redoStack = [];
    }

    function mutate(callback: var): void {
        if (!hasDocument)
            return;
        pushUndo();
        const next = clone(document);
        callback(next);
        document = next;
        dirty = authoredJson(next) !== savedDocumentJson;
    }

    function undo(): void {
        if (!canUndo)
            return;
        const previous = undoStack.slice();
        const target = previous.pop();
        const future = redoStack.slice();
        future.push(clone(document));
        undoStack = previous;
        redoStack = future;
        document = target;
        dirty = authoredJson(target) !== savedDocumentJson;
        reconcileSelection();
    }

    function redo(): void {
        if (!canRedo)
            return;
        const future = redoStack.slice();
        const target = future.pop();
        const previous = undoStack.slice();
        previous.push(clone(document));
        undoStack = previous;
        redoStack = future;
        document = target;
        dirty = authoredJson(target) !== savedDocumentJson;
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
        const revision = dirty ? saveRevision() : document;
        if (revision)
            soundAssemblyController.preparePreview(revision);
    }

    function exportMix(): void {
        const revision = dirty ? saveRevision() : document;
        if (!revision)
            return;
        exportDialog.revision = clone(revision);
        exportDialog.open();
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
                    return ({trackIndex: trackIndex, clipIndex: clipIndex});
            }
        }
        return null;
    }

    function reconcileSelection(): void {
        const location = clipLocation(selectedClipId);
        if (location !== null) {
            selectedTrackIndex = location.trackIndex;
            return;
        }
        selectedClipId = "";
        for (let trackIndex = 0; trackIndex < tracks.length; ++trackIndex) {
            if (tracks[trackIndex].clips.length > 0) {
                selectedTrackIndex = trackIndex;
                selectedClipId = tracks[trackIndex].clips[0].id;
                return;
            }
        }
        selectedTrackIndex = tracks.length > 0 ? 0 : -1;
    }

    function setTrackValue(trackIndex: int, key: string, value: var): void {
        mutate(next => next.tracks[trackIndex][key] = value);
    }

    function deleteTrack(trackIndex: int): void {
        if (trackIndex < 0 || trackIndex >= tracks.length || tracks.length <= 1
                || totalClipCount() <= tracks[trackIndex].clips.length)
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
        const location = clipLocation(clipId);
        if (location === null)
            return;
        selectedClipId = clipId;
        selectedTrackIndex = trackIndex;
        mutate(next => next.tracks[location.trackIndex].clips[location.clipIndex].timelineStartMillis = Math.max(0, Math.round(timelineStartMillis)));
    }

    function trimClip(trackIndex: int, clipId: string, sourceStart: real, sourceEnd: real, timelineStart: real): void {
        const location = clipLocation(clipId);
        if (location === null || sourceEnd - sourceStart < 10)
            return;
        selectedClipId = clipId;
        selectedTrackIndex = trackIndex;
        mutate(next => {
            const clip = next.tracks[location.trackIndex].clips[location.clipIndex];
            clip.sourceStartMillis = Math.round(sourceStart);
            clip.sourceEndMillis = Math.round(sourceEnd);
            clip.timelineStartMillis = Math.max(0, Math.round(timelineStart));
            const duration = clip.sourceEndMillis - clip.sourceStartMillis;
            clip.fadeInMillis = Math.min(clip.fadeInMillis, duration);
            clip.fadeOutMillis = Math.min(clip.fadeOutMillis, duration - clip.fadeInMillis);
        });
    }

    function deleteSelectedClip(): void {
        const location = clipLocation(selectedClipId);
        if (location === null || totalClipCount() <= 1)
            return;
        mutate(next => next.tracks[location.trackIndex].clips.splice(location.clipIndex, 1));
        reconcileSelection();
    }

    function duplicateSelectedClip(): void {
        const location = clipLocation(selectedClipId);
        if (location === null || totalClipCount() >= 256)
            return;
        const newId = backend.newAssemblyObjectId();
        mutate(next => {
            const copy = clone(next.tracks[location.trackIndex].clips[location.clipIndex]);
            copy.id = newId;
            copy.timelineStartMillis += 250;
            next.tracks[location.trackIndex].clips.splice(location.clipIndex + 1, 0, copy);
        });
        selectedClipId = newId;
    }

    function splitSelectedClip(): void {
        const location = clipLocation(selectedClipId);
        if (location === null || totalClipCount() >= 256)
            return;
        const clip = selectedClip;
        const end = clip.timelineStartMillis + clip.sourceEndMillis - clip.sourceStartMillis;
        if (playheadMillis <= clip.timelineStartMillis + 10 || playheadMillis >= end - 10) {
            presentError(qsTr("Place the playhead inside the selected clip before splitting."));
            return;
        }
        const newId = backend.newAssemblyObjectId();
        mutate(next => {
            const first = next.tracks[location.trackIndex].clips[location.clipIndex];
            const second = clone(first);
            const offset = Math.round(playheadMillis - first.timelineStartMillis);
            const splitSource = first.sourceStartMillis + offset;
            first.sourceEndMillis = splitSource;
            first.fadeOutMillis = Math.min(first.fadeOutMillis, first.sourceEndMillis - first.sourceStartMillis);
            second.id = newId;
            second.sourceStartMillis = splitSource;
            second.timelineStartMillis = Math.round(playheadMillis);
            second.fadeInMillis = Math.min(second.fadeInMillis, second.sourceEndMillis - second.sourceStartMillis);
            next.tracks[location.trackIndex].clips.splice(location.clipIndex + 1, 0, second);
        });
        selectedClipId = newId;
    }

    function moveSelectedClipToTrack(delta: int): void {
        const location = clipLocation(selectedClipId);
        if (location === null)
            return;
        const target = location.trackIndex + delta;
        if (target < 0 || target >= tracks.length)
            return;
        mutate(next => {
            const clip = next.tracks[location.trackIndex].clips.splice(location.clipIndex, 1)[0];
            next.tracks[target].clips.push(clip);
        });
        selectedTrackIndex = target;
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

    function addLibraryAsset(asset: var): void {
        if (!hasDocument || !asset || totalClipCount() >= 256)
            return;
        const targetTrack = selectedTrackIndex >= 0 ? selectedTrackIndex : 0;
        const duration = linearAssetDuration(asset);
        if (duration <= 0) {
            presentError(qsTr("This sound has no usable duration."));
            return;
        }
        const clipId = backend.newAssemblyObjectId();
        mutate(next => next.tracks[targetTrack].clips.push({
            id: clipId,
            assetId: asset.id,
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
        addSoundDialog.close();
    }

    function formatTime(millis: real): string {
        const totalSeconds = Math.max(0, Math.floor(millis / 1000));
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor(totalSeconds % 3600 / 60);
        const seconds = totalSeconds % 60;
        return hours > 0
            ? String(hours).padStart(2, "0") + ":" + String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0")
            : String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0");
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.preferredWidth: 252
            Layout.fillHeight: true
            color: Theme.panel
            border.width: 1
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Assemblies")
                        color: Theme.textPrimary
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                    }

                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/refresh.svg"
                        toolTipText: qsTr("Refresh assemblies")
                        buttonSize: 28
                        iconSize: 15
                        onClicked: workspace.refreshAssemblies()
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Create from one or more selected Library sounds. Each clip keeps its exact source version.")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: workspace.assemblies
                    spacing: 6
                    clip: true

                    delegate: Rectangle {
                        required property var modelData
                        width: ListView.view.width
                        height: 72
                        radius: 8
                        color: workspace.hasDocument && workspace.document.id === modelData.assemblyId
                            ? Theme.surfaceSelected : assemblyTap.hovered ? Theme.buttonGhostHover : Theme.transparent
                        border.width: 1
                        border.color: workspace.hasDocument && workspace.document.id === modelData.assemblyId
                            ? Theme.accentBorder : Theme.border

                        Column {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.margins: 10
                            spacing: 4

                            Text {
                                width: parent.width
                                text: modelData.name
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }

                            Text {
                                text: qsTr("%1 tracks · %2 clips · v%3").arg(modelData.trackCount).arg(modelData.clipCount).arg(modelData.revisionNumber)
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontMeta
                            }
                        }

                        HoverHandler { id: assemblyTap }
                        TapHandler { onTapped: workspace.openAssembly(modelData.assemblyId) }
                    }
                }
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

                    TextField {
                        Layout.preferredWidth: 250
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
                            workspace.libraryAssets = backend.listAssets();
                            addSoundDialog.open();
                        }
                    }

                    EchoButton {
                        text: qsTr("Add track")
                        ghost: true
                        enabled: workspace.hasDocument && workspace.tracks.length < 8
                        onClicked: workspace.addTrack()
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        visible: soundAssemblyController.running
                        text: qsTr("Preparing %1%").arg(Math.round(soundAssemblyController.progress * 100))
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }

                    EchoButton {
                        text: soundAssemblyController.running ? qsTr("Cancel")
                            : player.playing && soundAssemblyController.hasPreview ? qsTr("Pause") : qsTr("Preview")
                        enabled: workspace.hasDocument
                        onClicked: {
                            if (soundAssemblyController.running) {
                                soundAssemblyController.cancel();
                            } else if (soundAssemblyController.hasPreview && !workspace.dirty) {
                                if (player.active)
                                    player.togglePause();
                                else
                                    player.play(soundAssemblyController.previewPath);
                            } else {
                                workspace.preview();
                            }
                        }
                    }

                    EchoButton {
                        text: qsTr("Mixdown…")
                        enabled: workspace.hasDocument && !soundAssemblyController.running
                        onClicked: workspace.exportMix()
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
                        Layout.preferredHeight: 34
                        color: Theme.panelInset
                        border.width: 1
                        border.color: Theme.border

                        Item {
                            anchors.fill: parent

                            Repeater {
                                model: Math.min(workspace.tickCount, 1500)

                                delegate: Item {
                                    required property int index
                                    x: workspace.trackHeaderWidth + index * workspace.tickStepSeconds * workspace.pixelsPerSecond
                                    width: 1
                                    height: parent.height

                                    Rectangle {
                                        anchors.bottom: parent.bottom
                                        width: 1
                                        height: 8
                                        color: Theme.borderStrong
                                    }

                                    Text {
                                        anchors.left: parent.left
                                        anchors.leftMargin: 4
                                        anchors.top: parent.top
                                        anchors.topMargin: 3
                                        text: workspace.formatTime(index * workspace.tickStepSeconds * 1000)
                                        color: Theme.textMuted
                                        font.pixelSize: 9
                                    }
                                }
                            }
                        }
                    }

                    Flickable {
                        id: timelineFlick
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        contentWidth: workspace.trackHeaderWidth + workspace.timelineWidth
                        contentHeight: Math.max(height, trackColumn.height)
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds

                        Column {
                            id: trackColumn
                            width: timelineFlick.contentWidth
                            spacing: 2

                            Repeater {
                                model: workspace.tracks

                                delegate: SoundAssemblyTrack {
                                    required property var modelData
                                    required property int index
                                    width: trackColumn.width
                                    track: modelData
                                    trackIndex: index
                                    pixelsPerSecond: workspace.pixelsPerSecond
                                    playheadMillis: workspace.playheadMillis
                                    selectedClipId: workspace.selectedClipId
                                    canDeleteTrack: workspace.tracks.length > 1
                                        && workspace.totalClipCount() > modelData.clips.length
                                    headerWidth: workspace.trackHeaderWidth
                                    timelineWidth: workspace.timelineWidth
                                    onTrackValueRequested: workspace.setTrackValue(trackIndex, key, value)
                                    onTrackDeleteRequested: workspace.deleteTrack(trackIndex)
                                    onClipSelected: function (trackIndex, clipId) {
                                        workspace.selectedTrackIndex = trackIndex;
                                        workspace.selectedClipId = clipId;
                                    }
                                    onClipMoveRequested: workspace.moveClip(trackIndex, clipId, timelineStartMillis)
                                    onClipTrimRequested: workspace.trimClip(trackIndex, clipId, sourceStartMillis, sourceEndMillis, timelineStartMillis)
                                }
                            }
                        }

                        ScrollBar.horizontal: ScrollBar {}
                        ScrollBar.vertical: ScrollBar {}
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 54
                        color: Theme.chrome
                        border.width: 1
                        border.color: Theme.border

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 14
                            anchors.rightMargin: 14
                            spacing: 10

                            Text {
                                text: workspace.formatTime(workspace.playheadMillis)
                                color: Theme.textPrimary
                                font.family: "Menlo"
                                font.pixelSize: Theme.fontBody
                                Layout.preferredWidth: 70
                            }

                            Slider {
                                Layout.fillWidth: true
                                from: 0
                                to: Math.max(1, workspace.durationMillis)
                                value: workspace.playheadMillis
                                onMoved: workspace.playheadMillis = value
                            }

                            Text {
                                text: qsTr("Zoom")
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontMeta
                            }

                            Slider {
                                Layout.preferredWidth: 120
                                from: 25
                                to: 240
                                value: workspace.pixelsPerSecond
                                onMoved: workspace.pixelsPerSecond = value
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 310
                    Layout.fillHeight: true
                    color: Theme.panel
                    border.width: 1
                    border.color: Theme.border

                    ScrollView {
                        anchors.fill: parent
                        contentWidth: availableWidth

                        ColumnLayout {
                            width: parent.width
                            spacing: 12

                            Text {
                                Layout.fillWidth: true
                                Layout.margins: 14
                                Layout.bottomMargin: 0
                                text: workspace.selectedClip !== null ? qsTr("Clip inspector") : qsTr("Master output")
                                color: Theme.textPrimary
                                font.pixelSize: 14
                                font.weight: Font.DemiBold
                            }

                            ColumnLayout {
                                visible: workspace.selectedClip !== null
                                Layout.fillWidth: true
                                Layout.leftMargin: 14
                                Layout.rightMargin: 14
                                spacing: 9

                                Text {
                                    Layout.fillWidth: true
                                    text: workspace.selectedClip !== null ? workspace.selectedClip.assetId : ""
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.fontMeta
                                    elide: Text.ElideMiddle
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    EchoButton { text: qsTr("Split"); ghost: true; onClicked: workspace.splitSelectedClip() }
                                    EchoButton { text: qsTr("Duplicate"); ghost: true; onClicked: workspace.duplicateSelectedClip() }
                                    EchoButton {
                                        text: qsTr("Delete")
                                        ghost: true
                                        enabled: workspace.totalClipCount() > 1
                                        onClicked: workspace.deleteSelectedClip()
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    EchoButton {
                                        text: qsTr("Track ↑")
                                        ghost: true
                                        enabled: workspace.selectedTrackIndex > 0
                                        onClicked: workspace.moveSelectedClipToTrack(-1)
                                    }
                                    EchoButton {
                                        text: qsTr("Track ↓")
                                        ghost: true
                                        enabled: workspace.selectedTrackIndex >= 0 && workspace.selectedTrackIndex < workspace.tracks.length - 1
                                        onClicked: workspace.moveSelectedClipToTrack(1)
                                    }
                                }

                                Label { text: qsTr("Clip gain") }
                                Slider {
                                    Layout.fillWidth: true
                                    from: -2400; to: 1200; stepSize: 50
                                    value: workspace.selectedClip !== null ? workspace.selectedClip.gainCentibels : 0
                                    onPressedChanged: {
                                        if (!pressed)
                                            workspace.setClipValue("gainCentibels", Math.round(value));
                                    }
                                }

                                Label { text: qsTr("Clip pan") }
                                Slider {
                                    Layout.fillWidth: true
                                    from: -100; to: 100; stepSize: 1
                                    value: workspace.selectedClip !== null ? workspace.selectedClip.panPercent : 0
                                    onPressedChanged: {
                                        if (!pressed)
                                            workspace.setClipValue("panPercent", Math.round(value));
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Fade in (ms)"); Layout.fillWidth: true }
                                    SpinBox {
                                        from: 0
                                        to: workspace.selectedClip !== null ? workspace.selectedClip.sourceEndMillis - workspace.selectedClip.sourceStartMillis : 0
                                        value: workspace.selectedClip !== null ? workspace.selectedClip.fadeInMillis : 0
                                        editable: true
                                        onValueModified: workspace.setClipValue("fadeInMillis", Math.min(value, workspace.selectedClip.sourceEndMillis - workspace.selectedClip.sourceStartMillis - workspace.selectedClip.fadeOutMillis))
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Fade out (ms)"); Layout.fillWidth: true }
                                    SpinBox {
                                        from: 0
                                        to: workspace.selectedClip !== null ? workspace.selectedClip.sourceEndMillis - workspace.selectedClip.sourceStartMillis : 0
                                        value: workspace.selectedClip !== null ? workspace.selectedClip.fadeOutMillis : 0
                                        editable: true
                                        onValueModified: workspace.setClipValue("fadeOutMillis", Math.min(value, workspace.selectedClip.sourceEndMillis - workspace.selectedClip.sourceStartMillis - workspace.selectedClip.fadeInMillis))
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Fade-in curve"); Layout.fillWidth: true }
                                    ComboBox {
                                        Layout.preferredWidth: 128
                                        model: [qsTr("Linear"), qsTr("Smooth"), qsTr("Equal power")]
                                        currentIndex: workspace.selectedClip !== null
                                            ? workspace.fadeCurveIndex(workspace.selectedClip.fadeInCurve) : 0
                                        onActivated: workspace.setClipValue("fadeInCurve", workspace.fadeCurveValue(currentIndex))
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Fade-out curve"); Layout.fillWidth: true }
                                    ComboBox {
                                        Layout.preferredWidth: 128
                                        model: [qsTr("Linear"), qsTr("Smooth"), qsTr("Equal power")]
                                        currentIndex: workspace.selectedClip !== null
                                            ? workspace.fadeCurveIndex(workspace.selectedClip.fadeOutCurve) : 0
                                        onActivated: workspace.setClipValue("fadeOutCurve", workspace.fadeCurveValue(currentIndex))
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Muted"); Layout.fillWidth: true }
                                    Switch {
                                        checked: workspace.selectedClip !== null && workspace.selectedClip.muted
                                        onToggled: workspace.setClipValue("muted", checked)
                                    }
                                }

                                Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: 14
                                Layout.rightMargin: 14
                                Layout.bottomMargin: 14
                                spacing: 9

                                Label { text: qsTr("Master gain") }
                                Slider {
                                    Layout.fillWidth: true
                                    from: -2400; to: 1200; stepSize: 50
                                    value: workspace.hasDocument ? workspace.document.master.gainCentibels : 0
                                    onPressedChanged: {
                                        if (!pressed)
                                            workspace.setMasterValue("gainCentibels", Math.round(value));
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Limiter"); Layout.fillWidth: true }
                                    Switch {
                                        checked: workspace.hasDocument && workspace.document.master.limiterEnabled
                                        onToggled: workspace.setMasterValue("limiterEnabled", checked)
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    Label { text: qsTr("Ceiling (0.01 dB)"); Layout.fillWidth: true }
                                    SpinBox {
                                        from: -600; to: 0
                                        value: workspace.hasDocument ? workspace.document.master.limiterCeilingCentibels : -100
                                        editable: true
                                        onValueModified: workspace.setMasterValue("limiterCeilingCentibels", value)
                                    }
                                }

                                Text {
                                    Layout.fillWidth: true
                                    visible: soundAssemblyController.hasResult
                                    text: qsTr("Last mix: %1 LUFS · %2 dBTP")
                                        .arg(soundAssemblyController.integratedLufs.toFixed(1))
                                        .arg(soundAssemblyController.truePeakDbtp.toFixed(1))
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    wrapMode: Text.WordWrap
                                }
                            }
                        }
                    }
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

    Dialog {
        id: addSoundDialog
        anchors.centerIn: parent
        modal: true
        width: 520
        height: 560
        title: qsTr("Add Library sound")
        standardButtons: Dialog.Close

        ListView {
            anchors.fill: parent
            model: workspace.libraryAssets
            clip: true
            spacing: 4

            delegate: Rectangle {
                required property var modelData
                width: ListView.view.width
                height: 58
                radius: 7
                color: assetHover.hovered ? Theme.buttonGhostHover : Theme.transparent

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10

                    ColumnLayout {
                        Layout.fillWidth: true
                        Text {
                            Layout.fillWidth: true
                            text: modelData.soundCaption || modelData.path.split("/").pop()
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            elide: Text.ElideRight
                        }
                        Text {
                            text: (workspace.linearAssetDuration(modelData) / 1000).toFixed(2) + qsTr(" s")
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontMeta
                        }
                    }

                    EchoButton {
                        text: qsTr("Add")
                        enabled: modelData.pathStatus === "present"
                        onClicked: workspace.addLibraryAsset(modelData)
                    }
                }
                HoverHandler { id: assetHover }
            }
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
        visible: workspace.errorText.length > 0 || soundAssemblyController.errorText.length > 0
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            radius: Theme.controlRadius
            color: Theme.warningSurface
            border.width: 1
            border.color: Theme.warningText
        }

        contentItem: Text {
            id: errorLabel
            text: workspace.errorText.length > 0 ? workspace.errorText : soundAssemblyController.errorText
            color: Theme.warningText
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    Timer {
        id: errorTimer
        interval: 5000
        onTriggered: workspace.errorText = ""
    }

    Connections {
        target: backend
        function onSoundAssembliesChanged(): void { workspace.refreshAssemblies(); }
    }

    Component.onCompleted: refreshAssemblies()
}
