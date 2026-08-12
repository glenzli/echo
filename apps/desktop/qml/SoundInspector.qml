//! Selected-sound quick inspector. It owns lightweight audition, text and
//! metadata presentation; the complete playback workspace is opened explicitly.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: inspector

    required property var asset
    required property var jobStats
    property var waveformLevels: []
    property var longAudioChapters: []
    property string loadedPath: ""
    property bool calibratingMetadata: false

    readonly property bool hasAsset: asset !== null && asset !== undefined
    readonly property string analysisStage: hasAsset ? String(asset.analysisStage || "text") : "text"
    readonly property string analysisState: hasAsset ? String(asset.analysisState || "missing") : "missing"
    readonly property string analysisRecovery: hasAsset ? String(asset.analysisRecovery || "none") : "none"
    readonly property int analysisProgress: hasAsset ? Number(asset.analysisProgress || 0) : 0

    signal affinityRequested(var asset, bool liked, int rating)
    signal retryAnalysisRequested(var asset)

    color: Theme.panelRaised

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function compactText(text: string, maximum: int): string {
        const normalized = String(text || "").replace(/\s+/g, " ").trim();
        return normalized.length <= maximum ? normalized : normalized.substring(0, maximum).trim() + "…";
    }

    function titleFor(selected: var): string {
        if (!selected) {
            return "";
        }
        if (selected.soundCaption.length > 0) {
            return compactText(selected.soundCaption, 68);
        }
        if (selected.sourceTitle.length > 0) {
            return compactText(selected.sourceTitle, 68);
        }
        return fileName(selected.path);
    }

    function formatDuration(millis: double): string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown length");
        }
        const totalSeconds = Math.floor(millis / 1000);
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = totalSeconds % 60;
        if (hours > 0) {
            return hours + ":" + String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0");
        }
        return minutes + ":" + String(seconds).padStart(2, "0");
    }

    function formatPosition(millis: double): string {
        return formatDuration(Math.max(1, millis));
    }

    function formatDate(millis: double): string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown");
        }
        return new Date(millis).toLocaleString(Qt.locale(), Locale.ShortFormat);
    }

    function refresh(): void {
        waveformLevels = [];
        longAudioChapters = [];
        if (!asset || !asset.id) {
            return;
        }
        if (asset.pathStatus !== "missing") {
            waveformLevels = backend.waveformForAsset(asset.id);
        }
        longAudioChapters = backend.longAudioChaptersForAsset(asset.id);
    }

    function playSelected(): void {
        if (!asset || asset.pathStatus === "missing") {
            return;
        }
        if (loadedPath !== asset.path) {
            player.play(asset.path);
            loadedPath = asset.path;
            const resume = listeningProgress.resumePositionMillis;
            if (resume > 0 && resume < Number(asset.durationMillis))
                player.seek(resume);
        } else {
            player.togglePause();
        }
    }

    ListeningProgressTracker {
        id: listeningProgress
        asset: inspector.asset
        loadedPath: inspector.loadedPath
        playbackStartMillis: 0
        playbackEndMillis: inspector.hasAsset ? Number(inspector.asset.durationMillis) : 0
        active: inspector.visible
    }

    function keywordTags(): var {
        const values = [];
        if (!asset) {
            return values;
        }
        for (let index = 0; index < asset.keywords.length; ++index) {
            values.push(String(asset.keywords[index]));
        }
        return values;
    }

    function isCalibrated(field: string): bool {
        if (!asset || !asset.calibratedFields)
            return false;
        for (const value of asset.calibratedFields) {
            if (String(value) === field)
                return true;
        }
        return false;
    }

    function hasCalibratedFields(fields: var): bool {
        for (const field of fields) {
            if (isCalibrated(field))
                return true;
        }
        return false;
    }

    function analysisStageLabel(): string {
        if (!asset)
            return "";
        if (analysisStage === "text")
            return qsTr("Text extraction");
        if (analysisStage === "sound_events")
            return qsTr("Sound event detection");
        if (analysisStage === "alignment")
            return qsTr("Speech alignment");
        if (analysisStage === "contextual")
            return qsTr("Context understanding");
        if (analysisStage === "long_audio")
            return qsTr("Long recording analysis");
        return qsTr("Analysis complete");
    }

    function analysisMessage(): string {
        if (!asset)
            return "";
        if (analysisRecovery === "source")
            return qsTr("Reconnect the Original before retrying analysis.");
        if (analysisRecovery === "automatic")
            return qsTr("Echo will resume this automatically when Infer Runtime is available.");
        if (analysisRecovery === "manual")
            return qsTr("This stage requires a manual retry. Completed stages are preserved.");
        if (analysisState === "running")
            return qsTr("Echo is analyzing this sound in the background.");
        if (analysisState === "pending" || analysisState === "missing")
            return qsTr("This sound is waiting in the background analysis queue.");
        if (analysisStage === "sound_events")
            return qsTr("No speech was detected; Echo is identifying audible events.");
        return qsTr("The current analysis pipeline is complete.");
    }

    onAssetChanged: {
        calibratingMetadata = false;
        Qt.callLater(refresh);
    }

    Connections {
        target: backend
        function onAssetsChanged(): void {
            inspector.refresh();
        }
    }

    Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        width: 1
        color: Theme.border
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 50, 270)
        spacing: 9
        visible: !inspector.hasAsset

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a sound")
            color: Theme.textPrimary
            font.pixelSize: 16
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Its waveform, text, source metadata, and AI analysis will appear here.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: inspector.hasAsset

        ScrollView {
            id: detailScroll

            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: detailScroll.availableWidth
                spacing: 11

                Item {
                    Layout.preferredHeight: 3
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 13
                    spacing: 8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset ? inspector.titleFor(inspector.asset) : ""
                            color: Theme.textPrimary
                            font.pixelSize: 17
                            font.weight: Font.Normal
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset ? inspector.fileName(inspector.asset.path) + " · " + inspector.formatDuration(inspector.asset.durationMillis) : ""
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideMiddle
                        }
                    }

                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/edit.svg"
                        selected: inspector.hasAsset && inspector.asset.calibratedFields.length > 0
                        toolTipText: qsTr("Edit sound information")
                        buttonSize: 28
                        iconSize: 15
                        onClicked: inspector.calibratingMetadata = true
                    }

                    Button {
                        id: inspectorLikeButton

                        implicitWidth: 28
                        implicitHeight: 28
                        padding: 0
                        onClicked: inspector.affinityRequested(inspector.asset, !inspector.asset.liked, inspector.asset.rating)

                        background: Rectangle {
                            radius: 7
                            color: inspector.hasAsset && inspector.asset.liked ? Theme.likeSurface : inspectorLikeButton.hovered ? Theme.surfaceSubtle : Theme.transparent
                        }

                        contentItem: EchoIcon {
                            source: inspector.hasAsset && inspector.asset.liked ? "qrc:/EchoDesktop/icons/heart-filled.svg" : "qrc:/EchoDesktop/icons/heart.svg"
                            color: inspector.hasAsset && inspector.asset.liked ? Theme.likeAccent : Theme.textDisabled
                            size: 17
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    spacing: 1

                    Text {
                        text: qsTr("Rating")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    Repeater {
                        model: 5

                        delegate: Button {
                            id: inspectorRatingButton

                            required property int index
                            implicitWidth: 21
                            implicitHeight: 24
                            padding: 0
                            focusPolicy: Qt.NoFocus
                            onClicked: inspector.affinityRequested(inspector.asset, inspector.asset.liked, inspector.asset.rating === index + 1 ? 0 : index + 1)

                            background: Rectangle {
                                radius: 5
                                color: inspectorRatingButton.hovered ? Theme.buttonGhostHover : Theme.transparent
                            }

                            contentItem: EchoIcon {
                                source: inspector.hasAsset && index < inspector.asset.rating ? "qrc:/EchoDesktop/icons/star-filled.svg" : "qrc:/EchoDesktop/icons/star.svg"
                                color: inspector.hasAsset && index < inspector.asset.rating ? Theme.ratingAccent : Theme.textDisabled
                                size: 14
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.leftMargin: 16
                    Layout.rightMargin: 16
                    Layout.preferredHeight: 126
                    radius: Theme.panelRadius
                    color: Theme.waveformSurface
                    border.color: Theme.borderStrong

                    WaveformView {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        anchors.topMargin: 18
                        anchors.bottomMargin: 30
                        levels: inspector.waveformLevels
                        fillColor: Theme.waveformFill
                        progressColor: Theme.waveformPlayed
                        centerLineColor: Theme.waveformCenter
                        renderMode: "bars"
                        barWidth: 2.4
                        barGap: 1.6
                        barRadius: 1.3
                        normalize: true
                        normalizationFloor: 0.24
                        amplitudeExponent: 0.8
                        progress: inspector.hasAsset && Number(inspector.asset.durationMillis) > 0 ? Math.max(0, Math.min(1, (inspector.loadedPath === inspector.asset.path ? player.position : listeningProgress.resumePositionMillis) / Number(inspector.asset.durationMillis))) : 0
                    }

                    EchoIconButton {
                        anchors.left: parent.left
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: 9
                        anchors.bottomMargin: 7
                        source: inspector.hasAsset && player.playing && inspector.loadedPath === inspector.asset.path ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                        toolTipText: player.playing ? qsTr("Pause") : qsTr("Play")
                        enabled: inspector.hasAsset && inspector.asset.pathStatus !== "missing"
                        buttonSize: 27
                        iconSize: 14
                        onClicked: inspector.playSelected()
                    }

                    Text {
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: 10
                        anchors.bottomMargin: 12
                        text: inspector.hasAsset && inspector.loadedPath === inspector.asset.path ? inspector.formatDuration(player.position) + " / " + inspector.formatDuration(player.duration) : listeningProgress.resumePositionMillis > 0 ? qsTr("Continue at %1").arg(inspector.formatDuration(listeningProgress.resumePositionMillis)) : inspector.hasAsset ? inspector.formatDuration(inspector.asset.durationMillis) : "—"
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    visible: inspector.hasAsset && (SoundSemantics.eventLabel(inspector.asset.eventType).length > 0 || inspector.asset.mood.length > 0 || SoundSemantics.languageLabel(inspector.asset.language).length > 0)
                    title: qsTr("SOUND ATTRIBUTES")

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 7

                        Text {
                            visible: inspector.hasCalibratedFields(["event_type", "mood", "language"])
                            text: qsTr("User calibrated")
                            color: Theme.accentSelectionText
                            font.pixelSize: Theme.fontMeta
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            columns: 2
                            columnSpacing: 9
                            rowSpacing: 7

                            Text {
                                visible: inspector.hasAsset && SoundSemantics.eventLabel(inspector.asset.eventType).length > 0
                                text: qsTr("Event")
                                color: Theme.textDisabled
                                font.pixelSize: Theme.fontMeta
                            }
                            SoundSemanticTag {
                                visible: inspector.hasAsset && SoundSemantics.eventLabel(inspector.asset.eventType).length > 0
                                Layout.fillWidth: true
                                text: inspector.hasAsset ? SoundSemantics.eventLabel(inspector.asset.eventType) : ""
                                kind: "event"
                                compact: false
                                maximumWidth: 180
                            }

                            Text {
                                visible: inspector.hasAsset && inspector.asset.mood.length > 0
                                text: qsTr("Mood")
                                color: Theme.textDisabled
                                font.pixelSize: Theme.fontMeta
                            }
                            SoundSemanticTag {
                                visible: inspector.hasAsset && inspector.asset.mood.length > 0
                                text: inspector.hasAsset ? inspector.asset.mood : ""
                                kind: "mood"
                                compact: false
                                maximumWidth: 180
                            }

                            Text {
                                visible: inspector.hasAsset && SoundSemantics.languageLabel(inspector.asset.language).length > 0
                                text: qsTr("Language")
                                color: Theme.textDisabled
                                font.pixelSize: Theme.fontMeta
                            }
                            SoundSemanticTag {
                                visible: inspector.hasAsset && SoundSemantics.languageLabel(inspector.asset.language).length > 0
                                text: inspector.hasAsset ? SoundSemantics.languageLabel(inspector.asset.language) : ""
                                kind: "language"
                                compact: false
                                maximumWidth: 180
                            }
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    visible: inspector.hasAsset && inspector.asset.keywords.length > 0
                    title: qsTr("KEYWORDS")

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text {
                            visible: inspector.isCalibrated("keywords")
                            text: qsTr("User calibrated")
                            color: Theme.accentSelectionText
                            font.pixelSize: Theme.fontMeta
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 5

                            Repeater {
                                model: inspector.keywordTags()

                                delegate: Rectangle {
                                    required property var modelData
                                    width: keywordText.implicitWidth + 12
                                    height: 23
                                    radius: 6
                                    color: Theme.accentSurfaceQuiet

                                    Text {
                                        id: keywordText
                                        anchors.centerIn: parent
                                        text: String(modelData)
                                        color: Theme.accentSelectionText
                                        font.pixelSize: Theme.fontMeta
                                    }
                                }
                            }
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    visible: inspector.longAudioChapters.length > 0
                    title: qsTr("CHAPTERS")

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Repeater {
                            model: inspector.longAudioChapters.filter(chapter => chapter.level === 0)

                            delegate: Rectangle {
                                required property var modelData
                                Layout.fillWidth: true
                                Layout.preferredHeight: 44
                                radius: 6
                                color: chapterMouse.containsMouse ? Theme.surfaceSubtle : Theme.transparent

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 7
                                    anchors.rightMargin: 7
                                    spacing: 8

                                    Text {
                                        text: inspector.formatPosition(modelData.startMillis)
                                        color: Theme.accent
                                        font.pixelSize: Theme.fontMeta
                                    }

                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 1

                                        Text {
                                            Layout.fillWidth: true
                                            text: modelData.soundCaption
                                            color: Theme.textPrimary
                                            font.pixelSize: Theme.fontBody
                                            elide: Text.ElideRight
                                        }

                                        Text {
                                            Layout.fillWidth: true
                                            visible: modelData.summary.length > 0
                                            text: modelData.summary
                                            color: Theme.textSecondary
                                            font.pixelSize: Theme.fontMeta
                                            elide: Text.ElideRight
                                        }
                                    }
                                }

                                MouseArea {
                                    id: chapterMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        if (inspector.loadedPath !== inspector.asset.path) {
                                            player.play(inspector.asset.path);
                                            inspector.loadedPath = inspector.asset.path;
                                        }
                                        player.seek(modelData.startMillis);
                                    }
                                }
                            }
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    visible: inspector.hasAsset && inspector.analysisState !== "done"
                    title: qsTr("ANALYSIS")

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 7

                            EchoIcon {
                                source: inspector.analysisRecovery === "source" ? "qrc:/EchoDesktop/icons/source-missing.svg" : inspector.analysisRecovery === "manual" ? "qrc:/EchoDesktop/icons/refresh.svg" : inspector.analysisState === "running" ? "qrc:/EchoDesktop/icons/sparkles.svg" : "qrc:/EchoDesktop/icons/clock.svg"
                                color: inspector.analysisRecovery === "source" || inspector.analysisRecovery === "manual" ? Theme.warningText : inspector.analysisState === "running" || inspector.analysisRecovery === "automatic" ? Theme.accent : Theme.textMuted
                                size: 15
                            }

                            Text {
                                Layout.fillWidth: true
                                text: inspector.analysisStageLabel()
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                            }

                            Text {
                                visible: inspector.analysisState === "running" && inspector.analysisProgress > 0
                                text: inspector.analysisProgress + "%"
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: inspector.analysisMessage()
                            color: inspector.analysisRecovery === "manual" || inspector.analysisRecovery === "source" ? Theme.warningText : Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            lineHeight: 1.3
                            wrapMode: Text.WordWrap
                        }

                        EchoButton {
                            visible: inspector.analysisRecovery === "manual"
                            text: qsTr("Retry analysis")
                            ghost: true
                            onClicked: inspector.retryAnalysisRequested(inspector.asset)
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    visible: inspector.hasAsset && (inspector.asset.textPreview.length > 0 || inspector.analysisStage === "text" && inspector.analysisState !== "done")
                    title: qsTr("TEXT")

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text {
                            visible: inspector.isCalibrated("transcript_text")
                            text: qsTr("User calibrated")
                            color: Theme.accentSelectionText
                            font.pixelSize: Theme.fontMeta
                        }

                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset && inspector.asset.textPreview.length > 0 ? inspector.asset.textPreview : inspector.analysisState === "pending" || inspector.analysisState === "running" ? qsTr("Echo is extracting text in the background…") : qsTr("No text has been extracted from this sound yet.")
                            color: inspector.hasAsset && inspector.asset.textPreview.length > 0 ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontBody
                            lineHeight: 1.35
                            wrapMode: Text.WordWrap
                            maximumLineCount: 6
                            elide: Text.ElideRight
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("SOURCE METADATA")

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 2
                        columnSpacing: 9
                        rowSpacing: 7

                        Text {
                            text: qsTr("Recorded")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset && inspector.asset.sourceCreatedAt.length > 0 ? inspector.asset.sourceCreatedAt : inspector.hasAsset ? inspector.formatDate(inspector.asset.recordedAtMillis > 0 ? inspector.asset.recordedAtMillis : inspector.asset.importedAtMillis) : ""
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text {
                            text: qsTr("Location")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset && inspector.asset.sourceLocation.length > 0 ? inspector.asset.sourceLocation : qsTr("Not embedded")
                            color: inspector.hasAsset && inspector.asset.sourceLocation.length > 0 ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text {
                            text: qsTr("Format")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset ? inspector.asset.codec.toUpperCase() + (inspector.asset.containerFormat.length > 0 ? " · " + inspector.asset.containerFormat : "") : ""
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }

                        Text {
                            text: qsTr("Audio")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                        }
                        Text {
                            Layout.fillWidth: true
                            text: inspector.hasAsset && inspector.asset.sampleRate > 0 ? qsTr("%1 Hz · %2 channel(s)").arg(inspector.asset.sampleRate).arg(inspector.asset.channelCount) : qsTr("Technical metadata pending")
                            color: inspector.hasAsset && inspector.asset.sampleRate > 0 ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }
                }

                InspectorSection {
                    Layout.fillWidth: true
                    title: qsTr("ORIGINAL")

                    Text {
                        Layout.fillWidth: true
                        text: inspector.hasAsset ? inspector.asset.path : ""
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        wrapMode: Text.WrapAnywhere
                        maximumLineCount: 3
                        elide: Text.ElideMiddle
                    }
                }

                Item {
                    Layout.preferredHeight: 10
                }
            }
        }
    }

    MetadataCalibrationEditor {
        id: metadataEditor
        anchors.fill: parent
        z: 10
        visible: inspector.hasAsset && inspector.calibratingMetadata
        asset: inspector.asset

        onCancelRequested: {
            saving = false;
            errorText = "";
            inspector.calibratingMetadata = false;
        }
        onSaveRequested: function (values) {
            const result = backend.calibrateAssetMetadata(inspector.asset.id, values.soundCaption, values.summary, values.eventType, values.mood, values.keywords, values.transcriptText, values.language, values.calibratedFields);
            saving = false;
            if (result.ok) {
                inspector.calibratingMetadata = false;
            } else {
                errorText = qsTr("The changes could not be saved");
            }
        }
    }
}
