//! Compact selected-recording metadata projection embedded in the primary
//! playback workspace. It owns display formatting without becoming a rail.

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: metadata

    property var asset: null

    implicitHeight: 82
    color: Theme.surfaceSubtle
    radius: Theme.controlRadius
    border.color: Theme.border

    function formatDuration(millis: int): string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown");
        }
        const totalSeconds = Math.floor(millis / 1000);
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = totalSeconds % 60;
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds;
    }

    function formatDate(millis: double): string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown");
        }
        return new Date(millis).toLocaleString(Qt.locale(), Locale.ShortFormat);
    }

    function insightTags(): string {
        if (asset === null) {
            return "";
        }
        const parts = [];
        const eventLabel = SoundSemantics.eventLabel(asset.eventType);
        if (eventLabel.length > 0) {
            parts.push(eventLabel);
        }
        if (asset.mood.length > 0) {
            parts.push(asset.mood);
        }
        const languageLabel = SoundSemantics.languageLabel(asset.language);
        if (languageLabel.length > 0) {
            parts.push(languageLabel);
        }
        const keywords = asset.keywords;
        for (let index = 0; index < keywords.length; ++index) {
            parts.push(String(keywords[index]));
        }
        return parts.join(" · ");
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        anchors.topMargin: 9
        anchors.bottomMargin: 9
        spacing: 7

        RowLayout {
            Layout.fillWidth: true
            spacing: 18

            Repeater {
                model: metadata.asset !== null ? [
                    {
                        label: qsTr("Format"),
                        value: metadata.asset.codec.length > 0 ? metadata.asset.codec.toUpperCase() : qsTr("Audio")
                    },
                    {
                        label: qsTr("Duration"),
                        value: metadata.formatDuration(metadata.asset.durationMillis)
                    },
                    {
                        label: metadata.asset.assemblyId ? qsTr("Collected") : metadata.asset.recordedAtMillis > 0 ? qsTr("Recorded") : qsTr("Imported"),
                        value: metadata.formatDate(metadata.asset.recordedAtMillis > 0 ? metadata.asset.recordedAtMillis : metadata.asset.importedAtMillis)
                    }
                ] : []

                delegate: RowLayout {
                    required property var modelData

                    spacing: 5

                    Text {
                        text: modelData.label
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontMeta
                    }

                    Text {
                        text: modelData.value
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: metadata.asset !== null && (metadata.asset.summary.length > 0 || metadata.insightTags().length > 0)
            spacing: 10

            Text {
                text: qsTr("Understanding")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                font.bold: true
            }

            Text {
                Layout.fillWidth: true
                visible: metadata.asset !== null && metadata.asset.summary.length > 0
                text: visible ? metadata.asset.summary : ""
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                elide: Text.ElideRight
            }

            Text {
                visible: metadata.insightTags().length > 0
                text: metadata.insightTags()
                color: Theme.accentSelectionText
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
                Layout.maximumWidth: metadata.width * 0.34
            }
        }
    }
}
