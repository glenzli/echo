//! Collection-level continuous listening projection. Sound Tape virtually
//! joins the current playable result order while playback continues to consume
//! one saved Asset revision at a time. It owns no persistent order or mix.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: tape

    required property var assets
    required property var selectedAsset
    required property var jobStats
    required property bool active

    property string tapeScope: "memories"
    property bool includeGeneratedSources: false
    property int excludedGeneratedCount: 0
    signal includeGeneratedRequested(bool include)
    property var entries: []
    property real totalDurationMillis: 0
    property real cueGlobalMillis: 0
    property bool advanceArmed: false

    readonly property int currentIndex: indexForAsset(selectedAsset)
    readonly property var currentEntry: currentIndex >= 0 ? entries[currentIndex] : null
    readonly property real currentLocalMillis: {
        if (currentEntry === null)
            return 0;
        if (playback.ownsActivePlayback())
            return Math.max(0, Math.min(currentEntry.durationMillis, Number(player.position) - currentEntry.trimStartMillis));
        return Math.max(0, Math.min(currentEntry.durationMillis, cueGlobalMillis - currentEntry.startMillis));
    }
    readonly property real tapePositionMillis: currentEntry === null ? 0
        : playback.ownsActivePlayback() ? currentEntry.startMillis + currentLocalMillis
        : Math.max(0, Math.min(totalDurationMillis, cueGlobalMillis))

    signal assetSelected(var asset)
    signal assetOpened(var asset)
    signal assemblyRequested(var assetIds, string layout)

    SourceDisclosureDialog { id: disclosure; catalogBackend: backend }
    color: Theme.window

    function trimStart(asset: var): real {
        return Math.max(0, Number(asset.trimStartMillis || 0));
    }

    function trimEnd(asset: var): real {
        const sourceDuration = Math.max(0, Number(asset.durationMillis || 0));
        const requested = Number(asset.trimEndMillis || 0);
        return Math.max(trimStart(asset), Math.min(sourceDuration, requested > 0 ? requested : sourceDuration));
    }

    function rebuild(): void {
        const projected = [];
        let cursor = 0;
        for (const asset of assets) {
            const start = trimStart(asset);
            const end = trimEnd(asset);
            if (asset.pathStatus === "missing" || end <= start)
                continue;
            projected.push({
                asset: asset,
                startMillis: cursor,
                durationMillis: end - start,
                trimStartMillis: start,
                trimEndMillis: end
            });
            cursor += end - start;
        }
        entries = projected;
        totalDurationMillis = cursor;
        syncSelection(true);
    }

    function indexForAsset(asset: var): int {
        if (!asset || !asset.id)
            return -1;
        for (let index = 0; index < entries.length; ++index) {
            if (entries[index].asset.id === asset.id)
                return index;
        }
        return -1;
    }

    function indexForPosition(millis: real): int {
        if (entries.length === 0)
            return -1;
        const bounded = Math.max(0, Math.min(Math.max(0, totalDurationMillis - 1), millis));
        let low = 0;
        let high = entries.length - 1;
        while (low <= high) {
            const middle = Math.floor((low + high) / 2);
            const entry = entries[middle];
            if (bounded < entry.startMillis) {
                high = middle - 1;
            } else if (bounded >= entry.startMillis + entry.durationMillis) {
                low = middle + 1;
            } else {
                return middle;
            }
        }
        return entries.length - 1;
    }

    function syncSelection(resetCue: bool): void {
        let index = currentIndex;
        if (index < 0 && active && entries.length > 0) {
            assetSelected(entries[0].asset);
            index = 0;
        }
        if (index < 0)
            return;
        if (resetCue)
            cueGlobalMillis = entries[index].startMillis;
        Qt.callLater(() => {
            segmentList.currentIndex = index;
            segmentList.positionViewAtIndex(index, ListView.Contain);
        });
    }

    function selectIndex(index: int, offsetMillis: real): void {
        if (index < 0 || index >= entries.length)
            return;
        advanceArmed = false;
        const entry = entries[index];
        if (!selectedAsset || selectedAsset.id !== entry.asset.id)
            assetSelected(entry.asset);
        cueGlobalMillis = entry.startMillis + Math.max(0, Math.min(entry.durationMillis, offsetMillis));
        Qt.callLater(() => {
            segmentList.currentIndex = index;
            segmentList.positionViewAtIndex(index, ListView.Contain);
        });
    }

    function startIndex(index: int, offsetMillis: real): void {
        if (index < 0 || index >= entries.length)
            return;
        selectIndex(index, offsetMillis);
        advanceArmed = true;
        const assetId = entries[index].asset.id;
        const sourceMillis = entries[index].trimStartMillis + Math.max(0, Math.min(entries[index].durationMillis, offsetMillis));
        Qt.callLater(() => {
            if (!tape.active || !tape.selectedAsset || tape.selectedAsset.id !== assetId) {
                tape.advanceArmed = false;
                return;
            }
            playback.playFrom(sourceMillis);
        });
    }

    function seekGlobal(millis: real): void {
        const index = indexForPosition(millis);
        if (index < 0)
            return;
        const entry = entries[index];
        const offset = Math.max(0, Math.min(entry.durationMillis, millis - entry.startMillis));
        const continuePlaying = player.playing && playback.ownsActivePlayback();
        if (index === currentIndex && playback.ownsActivePlayback()) {
            cueGlobalMillis = entry.startMillis + offset;
            player.seek(entry.trimStartMillis + offset);
            return;
        }
        selectIndex(index, offset);
        if (continuePlaying)
            startIndex(index, offset);
    }

    function toggleTapePlayback(): void {
        if (currentEntry === null)
            return;
        if (playback.ownsActivePlayback()) {
            player.togglePause();
            return;
        }
        startIndex(currentIndex, currentLocalMillis);
    }

    function formatDuration(millis: real): string {
        const totalSeconds = Math.max(0, Math.floor(millis / 1000));
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = totalSeconds % 60;
        if (hours > 0)
            return hours + ":" + (minutes < 10 ? "0" : "") + minutes + ":" + (seconds < 10 ? "0" : "") + seconds;
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds;
    }

    onAssetsChanged: Qt.callLater(rebuild)
    onSelectedAssetChanged: {
        advanceArmed = false;
        Qt.callLater(() => syncSelection(true));
    }
    onActiveChanged: {
        if (active)
            Qt.callLater(() => syncSelection(true));
        else
            advanceArmed = false;
    }
    Component.onCompleted: Qt.callLater(rebuild)

    Connections {
        target: player

        function onPositionChanged(): void {
            if (tape.active && playback.ownsActivePlayback())
                tape.advanceArmed = true;
        }

        function onStateChanged(): void {
            if (!tape.active)
                return;
            if (player.active)
                return;
            const entry = tape.currentEntry;
            if (tape.advanceArmed && entry !== null && Number(player.position) >= entry.trimEndMillis - 120) {
                tape.advanceArmed = false;
                tape.cueGlobalMillis = entry.startMillis + entry.durationMillis;
                if (tape.currentIndex + 1 < tape.entries.length)
                    tape.startIndex(tape.currentIndex + 1, 0);
                return;
            }
            tape.advanceArmed = false;
        }
    }

    AudioPlaybackWorkspace {
        id: playback
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: tapePanel.top
        asset: tape.selectedAsset
        active: tape.active
        jobStats: tape.jobStats
    }

    Rectangle {
        id: tapePanel
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 178
        color: Theme.chrome
        border.width: 1
        border.color: Theme.border

        ColumnLayout {
            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 14
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            spacing: 7

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1

                    Text {
                        Layout.fillWidth: true
                        text: qsTr("Sound tape")
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontSection
                        font.bold: true
                    }

                    Text {
                        Layout.fillWidth: true
                        text: tape.entries.length > 0
                            ? qsTr("%1 playable · %2 total · follows current result order").arg(tape.entries.length).arg(tape.formatDuration(tape.totalDurationMillis))
                            : qsTr("No playable sounds in the current results")
                        color: Theme.textMuted
                        font.pixelSize: 9
                        elide: Text.ElideRight
                    }
                }

                ColumnLayout {
                    spacing: 2
                    EchoCheckBox {
                        objectName: "tapeIncludeGenerated"
                        text: qsTr("Include AI-generated sources")
                        checked: tape.tapeScope !== "originals" && tape.includeGeneratedSources
                        enabled: tape.tapeScope !== "originals"
                        onToggled: tape.includeGeneratedRequested(checked)
                    }
                    Text {
                        text: tape.tapeScope === "originals" ? qsTr("Original tape excludes declared generated sources") : qsTr("%1 sounds excluded by source labels").arg(tape.excludedGeneratedCount)
                        color: Theme.textMuted; font.pixelSize: Theme.fontMeta
                    }
                }
                SourceDisclosureBadge { asset: tape.currentEntry ? tape.currentEntry.asset : null; editable: asset !== null; onActivated: disclosure.present(asset,0,0) }
                EchoButton {
                    text: qsTr("Add current to assembly")
                    ghost: true
                    enabled: tape.currentEntry !== null
                    onClicked: tape.assemblyRequested([tape.currentEntry.asset.id], "sequence")
                }
            }

            ListView {
                id: segmentList
                Layout.fillWidth: true
                Layout.preferredHeight: 84
                orientation: ListView.Horizontal
                spacing: 0
                clip: true
                cacheBuffer: 900
                model: tape.entries
                focus: tape.active
                highlightRangeMode: ListView.ApplyRange
                preferredHighlightBegin: Math.max(0, width * 0.34)
                preferredHighlightEnd: Math.max(0, width * 0.66)

                delegate: SoundTapeSegment {
                    required property var modelData

                    entry: modelData
                    selected: tape.currentEntry !== null && tape.currentEntry.asset.id === modelData.asset.id
                    progress: tape.tapePositionMillis <= modelData.startMillis ? 0
                        : tape.tapePositionMillis >= modelData.startMillis + modelData.durationMillis ? 1
                        : (tape.tapePositionMillis - modelData.startMillis) / modelData.durationMillis
                    onActivated: tape.selectIndex(index, 0)
                    onOpened: tape.assetOpened(modelData.asset)
                }

                ScrollBar.horizontal: ScrollBar {}
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 9

                EchoIconButton {
                    source: player.playing && playback.ownsActivePlayback() ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                    toolTipText: player.playing ? qsTr("Pause sound tape") : qsTr("Play sound tape")
                    accessibleName: toolTipText
                    enabled: tape.currentEntry !== null
                    buttonSize: 30
                    iconSize: 15
                    onClicked: tape.toggleTapePlayback()
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/stop.svg"
                    toolTipText: qsTr("Stop sound tape")
                    accessibleName: toolTipText
                    enabled: playback.ownsActivePlayback()
                    buttonSize: 30
                    iconSize: 14
                    onClicked: {
                        tape.advanceArmed = false;
                        player.stop();
                    }
                }

                Text {
                    text: tape.formatDuration(tape.tapePositionMillis)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                Slider {
                    id: tapePositionSlider
                    Layout.fillWidth: true
                    from: 0
                    to: Math.max(1, tape.totalDurationMillis)
                    value: tape.tapePositionMillis
                    enabled: tape.entries.length > 0
                    onMoved: tape.seekGlobal(value)
                }

                Text {
                    text: tape.formatDuration(tape.totalDurationMillis)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }
        }
    }
}
