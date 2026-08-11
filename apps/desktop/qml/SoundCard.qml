//! A sound's information-first visual surrogate for parallel Library browsing.
//! Density changes the evidence disclosed by the card while a compact, real,
//! width-selected waveform remains a secondary texture and playback landmark.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: card

    required property var entry
    required property bool selected
    required property string density
    required property int displayHeight
    property var waveformLevels: []

    signal activated(int modifiers)
    signal opened

    readonly property bool overview: density === "overview"
    readonly property bool rich: density === "rich"
    readonly property var tonePair: toneFor(entry.recordedAtMillis, Theme.effectiveDark)
    readonly property color toneStart: tonePair.start
    readonly property color toneEnd: tonePair.end
    readonly property string displayTitle: titleFor(entry)
    readonly property string displayPreview: String(entry.summary || "").trim().length > 0 ? entry.summary : entry.textPreview
    readonly property string keywordLine: keywordLineFor(entry)
    readonly property string sourceLine: sourceFor(entry)
    readonly property string evidenceLine: evidenceLineFor(entry)
    readonly property string displayEvent: SoundSemantics.eventLabel(entry.eventType)
    readonly property string displayLanguage: SoundSemantics.languageLabel(entry.language)
    readonly property bool hasSemanticLine: entry.mood.length > 0 || displayEvent.length > 0 || displayLanguage.length > 0 || keywordLine.length > 0
    readonly property bool hasPreview: String(displayPreview || "").trim().length > 0
    readonly property bool hasEvidence: evidenceLine.length > 0
    readonly property int waveformHeight: overview ? 48 : rich ? 62 : 52

    implicitWidth: 264
    implicitHeight: displayHeight
    radius: Theme.cardRadius
    color: Theme.panelRaised
    border.width: selected ? 2 : 1
    border.color: selected ? Theme.accent : cardHover.hovered ? Theme.borderStrong : Theme.border
    clip: true
    scale: cardMouse.pressed ? 0.996 : 1.0

    Behavior on scale {
        NumberAnimation {
            duration: 80
        }
    }
    Behavior on border.color {
        ColorAnimation {
            duration: 100
        }
    }

    Accessible.role: Accessible.ListItem
    Accessible.name: displayTitle
    Accessible.selected: selected

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function compactText(text: string, maximum: int): string {
        const normalized = String(text || "").replace(/\s+/g, " ").trim();
        if (normalized.length <= maximum) {
            return normalized;
        }
        return normalized.substring(0, maximum).trim() + "…";
    }

    function titleFor(asset: var): string {
        const maximum = rich ? 96 : overview ? 56 : 76;
        if (asset.soundCaption.length > 0) {
            return compactText(asset.soundCaption, maximum);
        }
        if (asset.sourceTitle.length > 0) {
            return compactText(asset.sourceTitle, maximum);
        }
        return fileName(asset.path);
    }

    function keywordLineFor(asset: var): string {
        const keywords = [];
        const keywordLimit = rich ? 3 : 2;
        for (let index = 0; index < Math.min(keywordLimit, asset.keywords.length); ++index) {
            keywords.push(String(asset.keywords[index]));
        }
        return keywords.join(" · ");
    }

    function sourceFor(asset: var): string {
        const parts = [];
        if (asset.sourceLocation.length > 0) {
            parts.push(asset.sourceLocation);
        }
        if (asset.codec.length > 0) {
            parts.push(asset.codec.toUpperCase());
        }
        if (asset.sampleRate > 0) {
            parts.push((asset.sampleRate / 1000).toFixed(asset.sampleRate % 1000 === 0 ? 0 : 1) + " kHz");
        }
        return parts.join(" · ");
    }

    function evidenceLineFor(asset: var): string {
        return sourceLine;
    }

    function formatDuration(millis: double): string {
        if (!millis || millis <= 0) {
            return "—";
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

    function formatDate(millis: double): string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown date");
        }
        return new Date(millis).toLocaleDateString(Qt.locale(), Locale.ShortFormat);
    }

    function toneFor(recordedAtMillis: double, dark: bool): var {
        const palettes = dark ? {
            dawn: {
                start: "#2b2e32",
                end: "#3a3d41"
            },
            day: {
                start: "#292d31",
                end: "#383c40"
            },
            dusk: {
                start: "#2d2d31",
                end: "#3e3d42"
            },
            night: {
                start: "#24282d",
                end: "#32373d"
            },
            unknown: {
                start: "#2a2d31",
                end: "#393d42"
            }
        } : {
            dawn: {
                start: "#e9ebed",
                end: "#f2f3f4"
            },
            day: {
                start: "#e7eaed",
                end: "#f1f3f5"
            },
            dusk: {
                start: "#eaeaec",
                end: "#f3f3f5"
            },
            night: {
                start: "#e5e8eb",
                end: "#eef0f2"
            },
            unknown: {
                start: "#e9ecef",
                end: "#f1f3f5"
            }
        };
        if (!recordedAtMillis || recordedAtMillis <= 0) {
            return palettes.unknown;
        }

        const hour = new Date(recordedAtMillis).getHours();
        if (hour >= 5 && hour < 9) {
            return palettes.dawn;
        }
        if (hour >= 9 && hour < 17) {
            return palettes.day;
        }
        if (hour >= 17 && hour < 21) {
            return palettes.dusk;
        }
        return palettes.night;
    }

    function loadWaveform(): void {
        if (entry === null || !entry.id || entry.pathStatus === "missing") {
            waveformLevels = [];
            return;
        }
        try {
            waveformLevels = backend.waveformForAsset(entry.id);
        } catch (error) {
            waveformLevels = [];
        }
    }

    Component.onCompleted: Qt.callLater(loadWaveform)

    HoverHandler {
        id: cardHover
    }

    Rectangle {
        anchors.fill: parent
        visible: cardHover.hovered && !card.selected
        color: Theme.accentSurfaceQuiet
        opacity: 0.28
        z: 5
    }

    MouseArea {
        id: cardMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        cursorShape: Qt.PointingHandCursor
        z: 6
        onClicked: mouse => card.activated(mouse.modifiers)
        onDoubleClicked: card.opened()
    }

    Rectangle {
        id: soundVisual

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: card.waveformHeight
        z: 1
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop {
                position: 0.0
                color: card.toneStart
            }
            GradientStop {
                position: 1.0
                color: card.toneEnd
            }
        }

        Repeater {
            model: card.overview ? 4 : card.rich ? 8 : 6

            Rectangle {
                required property int index
                x: index * soundVisual.width / (card.overview ? 4 : card.rich ? 8 : 6)
                width: 1
                height: soundVisual.height
                color: Theme.effectiveDark ? "#12ffffff" : "#0d000000"
            }
        }

        WaveformView {
            anchors.fill: parent
            anchors.leftMargin: card.rich ? 14 : 11
            anchors.rightMargin: card.rich ? 14 : 11
            anchors.topMargin: card.overview ? 8 : 9
            anchors.bottomMargin: card.overview ? 10 : 11
            levels: card.waveformLevels
            fillColor: Theme.waveformFill
            progressColor: Theme.waveformPlayed
            centerLineColor: Theme.waveformCenter
            renderMode: "bars"
            barWidth: card.rich ? 2.4 : 2.1
            barGap: card.rich ? 1.5 : 1.35
            barRadius: 1.2
            normalize: true
            normalizationFloor: 0.28
            amplitudeExponent: 0.76
            progress: 0
        }

        Rectangle {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.rightMargin: 8
            anchors.bottomMargin: 6
            width: durationText.implicitWidth + 10
            height: 18
            radius: 5
            color: Theme.effectiveDark ? "#70121518" : "#d6ffffff"

            Text {
                id: durationText
                anchors.centerIn: parent
                text: card.formatDuration(card.entry.durationMillis)
                color: Theme.effectiveDark ? "#f4f5f6" : "#343b42"
                font.pixelSize: 9
            }
        }

        Text {
            anchors.centerIn: parent
            visible: card.waveformLevels.length === 0
            text: card.entry.pathStatus === "missing" ? qsTr("Original missing") : qsTr("Preparing waveform…")
            color: Theme.effectiveDark ? "#d2d7dc" : "#59636d"
            font.pixelSize: Theme.fontMeta
        }
    }

    ColumnLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: soundVisual.bottom
        anchors.bottom: parent.bottom
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        anchors.topMargin: card.overview ? 8 : 9
        anchors.bottomMargin: 8
        spacing: 4
        z: 10

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: card.overview ? 31 : 35
            Layout.maximumHeight: Layout.preferredHeight
            spacing: 6

            Text {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignTop
                text: card.displayTitle
                color: Theme.textPrimary
                font.pixelSize: card.rich ? 13 : 12
                font.weight: Font.Normal
                lineHeight: 1.12
                maximumLineCount: 2
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
            }

            EchoIcon {
                Layout.alignment: Qt.AlignTop
                visible: card.entry.liked
                source: "qrc:/EchoDesktop/icons/heart-filled.svg"
                color: Theme.likeAccent
                size: 14
            }

            RowLayout {
                Layout.alignment: Qt.AlignTop
                visible: card.entry.rating > 0
                spacing: 2

                EchoIcon {
                    source: "qrc:/EchoDesktop/icons/star-filled.svg"
                    color: Theme.ratingAccent
                    size: 13
                }

                Text {
                    text: card.entry.rating
                    color: Theme.ratingAccent
                    font.pixelSize: 9
                    font.bold: true
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: card.hasSemanticLine
            Layout.preferredHeight: visible ? 20 : 0
            Layout.maximumHeight: Layout.preferredHeight
            spacing: 5

            SoundSemanticTag {
                text: card.entry.mood
                kind: "mood"
                maximumWidth: 78
            }

            SoundSemanticTag {
                text: card.displayEvent
                kind: "event"
                maximumWidth: 82
            }

            SoundSemanticTag {
                text: card.displayLanguage
                kind: "language"
                maximumWidth: 72
            }

            Text {
                Layout.fillWidth: true
                visible: card.keywordLine.length > 0
                text: card.keywordLine
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.preferredHeight: visible ? (card.rich ? 48 : 34) : 0
            Layout.maximumHeight: Layout.preferredHeight
            visible: !card.overview && card.hasPreview
            text: card.compactText(card.displayPreview, card.rich ? 150 : 90)
            color: Theme.textSecondary
            font.pixelSize: 11
            lineHeight: 1.22
            maximumLineCount: card.rich ? 3 : 2
            wrapMode: Text.WordWrap
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            Layout.preferredHeight: visible ? 13 : 0
            Layout.maximumHeight: Layout.preferredHeight
            visible: !card.overview && card.hasEvidence
            text: card.evidenceLine
            color: Theme.textMuted
            font.pixelSize: 9
            elide: Text.ElideRight
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignBottom
            Layout.topMargin: card.overview ? 0 : 1
            spacing: 5

            Text {
                Layout.fillWidth: true
                text: card.entry.sourceLocation.length > 0 ? card.entry.sourceLocation : card.formatDate(card.entry.recordedAtMillis > 0 ? card.entry.recordedAtMillis : card.entry.importedAtMillis)
                color: Theme.textDisabled
                font.pixelSize: 9
                elide: Text.ElideRight
            }

            Text {
                visible: card.rich && card.entry.sourceLocation.length > 0
                text: card.formatDate(card.entry.recordedAtMillis > 0 ? card.entry.recordedAtMillis : card.entry.importedAtMillis)
                color: Theme.textDisabled
                font.pixelSize: 9
            }
        }
    }
}
