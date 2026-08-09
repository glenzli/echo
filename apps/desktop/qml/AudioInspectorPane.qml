//! Contextual inspector for the selected recording. It intentionally excludes
//! waveform and transport so this narrower column never constrains playback.

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: inspector

    property var asset: null

    color: Theme.panel
    radius: Theme.panelRadius
    border.color: Theme.border
    clip: true

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function formatDuration(millis: int) : string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown")
        }
        const totalSeconds = Math.floor(millis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds
    }

    function formatDate(millis: double) : string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown")
        }
        return new Date(millis).toLocaleString(Qt.locale(), Locale.ShortFormat)
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 44, 220)
        spacing: 8
        visible: inspector.asset === null

        Text {
            Layout.fillWidth: true
            text: qsTr("Recording details")
            color: Theme.textPrimary
            font.pixelSize: 14
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a sound to inspect its original and understanding evidence.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        id: details

        anchors.fill: parent
        anchors.margins: 18
        visible: inspector.asset !== null
        spacing: 14

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Recording")
                hint: qsTr("Immutable original")
            }

            Text {
                Layout.fillWidth: true
                text: inspector.asset !== null ? inspector.fileName(inspector.asset.path) : ""
                color: Theme.textPrimary
                font.pixelSize: 15
                font.bold: true
                wrapMode: Text.WrapAnywhere
            }

            Text {
                Layout.fillWidth: true
                text: inspector.asset !== null ? inspector.asset.path : ""
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WrapAnywhere
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Properties")
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 12
                rowSpacing: 9

                Text { text: qsTr("Format"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                Text {
                    Layout.fillWidth: true
                    text: inspector.asset !== null && inspector.asset.codec.length > 0
                        ? inspector.asset.codec.toUpperCase() : qsTr("Audio")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    horizontalAlignment: Text.AlignRight
                    elide: Text.ElideRight
                }

                Text { text: qsTr("Duration"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                Text {
                    Layout.fillWidth: true
                    text: inspector.asset !== null
                        ? inspector.formatDuration(inspector.asset.durationMillis) : ""
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    horizontalAlignment: Text.AlignRight
                }

                Text { text: qsTr("Recorded"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                Text {
                    Layout.fillWidth: true
                    text: inspector.asset !== null
                        ? inspector.formatDate(inspector.asset.recordedAtMillis > 0
                            ? inspector.asset.recordedAtMillis
                            : inspector.asset.importedAtMillis) : ""
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    horizontalAlignment: Text.AlignRight
                    wrapMode: Text.WordWrap
                }

                Text { text: qsTr("Evidence"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                Text {
                    Layout.fillWidth: true
                    text: inspector.asset !== null && inspector.asset.maxLevel > 0
                        ? qsTr("Level %1").arg(inspector.asset.maxLevel)
                        : qsTr("Original only")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    horizontalAlignment: Text.AlignRight
                }

                Text { text: qsTr("Availability"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                Text {
                    Layout.fillWidth: true
                    text: inspector.asset !== null && inspector.asset.pathStatus === "missing"
                        ? qsTr("File missing") : qsTr("Available")
                    color: inspector.asset !== null && inspector.asset.pathStatus === "missing"
                        ? Theme.warningText : Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    horizontalAlignment: Text.AlignRight
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Understanding")
                hint: inspector.asset !== null && inspector.asset.maxLevel > 0
                    ? qsTr("Derived evidence") : qsTr("Not analyzed yet")
            }

            Text {
                Layout.fillWidth: true
                text: inspector.asset !== null && inspector.asset.summary.length > 0
                    ? inspector.asset.summary
                    : qsTr("No contextual understanding evidence is available for this recording.")
                color: inspector.asset !== null && inspector.asset.summary.length > 0
                    ? Theme.textPrimary : Theme.textSecondary
                font.pixelSize: Theme.fontBody
                wrapMode: Text.WordWrap
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8
                visible: inspector.asset !== null
                    && (inspector.asset.eventType.length > 0 || inspector.asset.mood.length > 0)

                RowLayout {
                    Layout.fillWidth: true
                    visible: inspector.asset !== null && inspector.asset.eventType.length > 0
                    Text { text: qsTr("Event"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                    Item { Layout.fillWidth: true }
                    Text { text: inspector.asset !== null ? inspector.asset.eventType : ""; color: Theme.textPrimary; font.pixelSize: Theme.fontBody }
                }

                RowLayout {
                    Layout.fillWidth: true
                    visible: inspector.asset !== null && inspector.asset.mood.length > 0
                    Text { text: qsTr("Mood"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                    Item { Layout.fillWidth: true }
                    Text { text: inspector.asset !== null ? inspector.asset.mood : ""; color: Theme.textPrimary; font.pixelSize: Theme.fontBody }
                }
            }

            Flow {
                Layout.fillWidth: true
                Layout.preferredHeight: childrenRect.height
                spacing: 6
                visible: inspector.asset !== null && inspector.asset.keywords.length > 0

                Repeater {
                    model: inspector.asset !== null ? inspector.asset.keywords : []

                    delegate: Rectangle {
                        required property var modelData
                        width: keywordText.implicitWidth + 14
                        height: 22
                        radius: 11
                        color: Theme.accentSurfaceQuiet

                        Text {
                            id: keywordText
                            anchors.centerIn: parent
                            text: modelData
                            color: Theme.accentSelectionText
                            font.pixelSize: Theme.fontMeta
                        }
                    }
                }
            }

            Item {
                Layout.fillHeight: true
            }
    }
}
