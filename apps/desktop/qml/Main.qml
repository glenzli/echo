//! Echo application shell: Audio Space is the first screen. Listen first,
//! edit second — selecting a recording opens the waveform and playback
//! controls below the list.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

ApplicationWindow {
    id: window

    width: 1080
    height: 720
    minimumWidth: 760
    minimumHeight: 480
    visible: true
    title: qsTr("Echo")

    color: Theme.background

    property var selectedAsset: null

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            color: Theme.surface
            border.color: Theme.divider

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 12

                Text {
                    text: qsTr("Echo")
                    color: Theme.textPrimary
                    font.pixelSize: 15
                    font.bold: true
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: backend.catalogPath
                    color: Theme.textSecondary
                    font.pixelSize: 11
                    elide: Text.ElideMiddle
                    Layout.maximumWidth: 280
                }

                EchoButton {
                    text: qsTr("Refresh")
                    onClicked: backend.refresh()
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.divider
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 24
                spacing: 16

                Text {
                    text: qsTr("Audio Space")
                    color: Theme.textPrimary
                    font.pixelSize: 22
                    font.bold: true
                }

                Text {
                    text: qsTr("%1 recordings in your library").arg(backend.assetCount)
                    color: Theme.textSecondary
                    font.pixelSize: 12
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Theme.surface
                    radius: 10
                    border.color: Theme.divider

                    ListView {
                        id: assetList

                        anchors.fill: parent
                        anchors.margins: 8
                        spacing: 4
                        clip: true
                        model: backend.listAssets()

                        delegate: Rectangle {
                            required property var modelData

                            width: assetList.width - 16
                            height: 56
                            radius: 8
                            color: selectedAsset !== null && selectedAsset.id === modelData.id
                                ? Qt.lighter(Theme.surfaceRaised, 1.12)
                                : Theme.surfaceRaised

                            MouseArea {
                                anchors.fill: parent
                                onClicked: {
                                    window.selectedAsset = modelData
                                    player.play(modelData.path)
                                }
                            }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 12
                                anchors.rightMargin: 12
                                spacing: 12

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 2

                                    Text {
                                        text: modelData.path
                                        color: Theme.textPrimary
                                        font.pixelSize: 13
                                        elide: Text.ElideMiddle
                                        Layout.fillWidth: true
                                    }

                                    Text {
                                        text: qsTr("%1 · %2 · level %3")
                                            .arg(modelData.codec)
                                            .arg(formatDuration(modelData.durationMillis))
                                            .arg(modelData.maxLevel)
                                        color: Theme.textSecondary
                                        font.pixelSize: 11
                                    }
                                }
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 196
                    visible: selectedAsset !== null
                    color: Theme.surface
                    radius: 10
                    border.color: Theme.divider

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 8

                        Text {
                            text: selectedAsset !== null ? selectedAsset.path : ""
                            color: Theme.textPrimary
                            font.pixelSize: 13
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }

                        WaveformView {
                            id: waveform

                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            levels: selectedAsset !== null
                                ? backend.waveformForAsset(selectedAsset.id)
                                : []
                            progress: player.duration > 0
                                ? player.position / player.duration
                                : 0.0
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            EchoButton {
                                text: player.isPlaying ? qsTr("Pause") : qsTr("Play")
                                enabled: selectedAsset !== null
                                onClicked: {
                                    if (player.isPlaying || player.isPaused) {
                                        player.togglePause()
                                    } else {
                                        player.play(selectedAsset.path)
                                    }
                                }
                            }

                            EchoButton {
                                text: qsTr("Stop")
                                enabled: player.duration > 0
                                onClicked: player.stop()
                            }

                            Text {
                                text: formatDuration(player.position) + " / "
                                    + formatDuration(player.duration)
                                color: Theme.textSecondary
                                font.pixelSize: 11
                            }

                            Item { Layout.fillWidth: true }

                            Text {
                                text: qsTr("Volume")
                                color: Theme.textSecondary
                                font.pixelSize: 11
                            }

                            Slider {
                                id: volumeSlider

                                from: 0
                                to: 1
                                value: player.volume
                                Layout.preferredWidth: 120
                                onMoved: player.volume = value
                            }
                        }

                        Slider {
                            id: seekSlider

                            Layout.fillWidth: true
                            from: 0
                            to: Math.max(1, player.duration)
                            value: player.duration > 0 ? player.position : 0
                            enabled: player.duration > 0
                            onMoved: player.seek(value)
                        }
                    }
                }
            }
        }
    }

    function formatDuration(millis: int) : string {
        if (millis <= 0) {
            return qsTr("unknown duration")
        }
        const totalSeconds = Math.floor(millis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds
    }
}
