//! Echo application shell: Audio Space is the first screen. The title bar and
//! toolbar are one fused chrome surface (window dragging over its empty
//! area); toolbar actions are icon-first.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import EchoDesktop

ApplicationWindow {
    id: window

    width: 1080
    height: 720
    minimumWidth: 760
    minimumHeight: 480
    visible: true
    // macOS paints its own title strip above the QML scene; the window title
    // would render a second "Echo" there, so it stays empty (Shadow contract).
    title: Qt.platform.os === "osx" ? "" : qsTr("Echo")

    // Keep the native frame and traffic-light controls; the toolbar paints
    // through the transparent title-bar strip (fused title bar + toolbar).
    flags: Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint

    color: Theme.window

    property var selectedAsset: null
    property bool appearancePinned: false

    EchoSettingsDialog {
        id: settingsDialog
    }

    // Fused title bar + toolbar: the QML scene is offset below the native
    // title strip (32 pt), so the toolbar is pulled up by that amount and
    // paints from the very top of the window, under the traffic lights.
    // The actions row (22 pt) then shares one line with the lights.
    Rectangle {
        id: titleBar

        anchors.left: parent.left
        anchors.right: parent.right
        height: 44
        y: window.visibility === Window.FullScreen ? 0 : -32
        color: Theme.chrome

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Math.max(
                SafeArea.margins.left,
                Qt.platform.os === "osx" && window.visibility !== Window.FullScreen ? 96 : 16
            )
            anchors.rightMargin: Math.max(
                SafeArea.margins.right,
                Qt.platform.os === "windows" ? 152 : 16
            )
            spacing: 8

            Text {
                text: qsTr("Echo")
                color: Theme.textPrimary
                font.pixelSize: 14
                font.bold: true
            }

            Text {
                text: qsTr("Audio Space")
                color: Theme.textSecondary
                font.pixelSize: 12
            }

            Item {
                Layout.fillWidth: true

                // Native window dragging over the empty chrome area.
                DragHandler {
                    acceptedButtons: Qt.LeftButton
                    target: null
                    onActiveChanged: {
                        if (active) {
                            window.startSystemMove()
                        }
                    }
                }
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/refresh.svg"
                toolTipText: qsTr("Refresh library")
                onClicked: backend.refresh()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/tune.svg"
                toolTipText: qsTr("Settings")
                onClicked: settingsDialog.open()
            }
        }
    }

    Item {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: titleBar.bottom
        anchors.bottom: parent.bottom

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 24
            spacing: 16

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Text {
                    text: qsTr("Audio Space")
                    color: Theme.textPrimary
                    font.pixelSize: 22
                    font.bold: true
                }

                Text {
                    text: qsTr("%1 recordings in your library").arg(backend.assetCount)
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontBody
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: backend.catalogPath
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideMiddle
                    Layout.maximumWidth: 260
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: Theme.panel
                radius: 10
                border.color: Theme.border

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
                            ? Theme.surfaceSelected
                            : Theme.panelRaised

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

                            EchoIcon {
                                source: "qrc:/EchoDesktop/icons/waveform.svg"
                                size: 18
                                color: Theme.textSecondary
                            }

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
                color: Theme.panel
                radius: 10
                border.color: Theme.border

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
                        spacing: 6

                        EchoIconButton {
                            source: player.isPlaying
                                ? "qrc:/EchoDesktop/icons/pause.svg"
                                : "qrc:/EchoDesktop/icons/play.svg"
                            toolTipText: player.isPlaying ? qsTr("Pause") : qsTr("Play")
                            enabled: selectedAsset !== null
                            buttonSize: 34
                            iconSize: 18
                            onClicked: {
                                if (player.isPlaying || player.isPaused) {
                                    player.togglePause()
                                } else {
                                    player.play(selectedAsset.path)
                                }
                            }
                        }

                        EchoIconButton {
                            source: "qrc:/EchoDesktop/icons/stop.svg"
                            toolTipText: qsTr("Stop")
                            enabled: player.duration > 0
                            buttonSize: 34
                            iconSize: 18
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
                            font.pixelSize: Theme.fontMeta
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

    // Appearance follows UiPreferences; the Theme singleton resolves the
    // palette from `uiPrefs.dark`.
    Component.onCompleted: {
        settingsDialog.uiPrefs = uiPrefs
        Theme.mode = uiPrefs.mode
        uiPrefs.modeChanged.connect(() => {
            Theme.mode = uiPrefs.mode
        })
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
