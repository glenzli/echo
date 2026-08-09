//! One-sound listening presentation with a horizontal filmstrip. It consumes
//! the same filtered asset projection as SoundWall, so collection, search and
//! advanced filter changes remain identical across both views.

import QtQuick
import QtQuick.Controls
import EchoDesktop

Rectangle {
    id: focusView

    required property var assets
    required property var selectedAsset
    required property var jobStats

    signal assetSelected(var asset)
    signal affinityRequested(var asset, bool liked, int rating)

    color: Theme.window

    function selectedIndex() : int {
        if (selectedAsset === null) {
            return -1
        }
        for (let index = 0; index < assets.length; ++index) {
            if (assets[index].id === selectedAsset.id) {
                return index
            }
        }
        return -1
    }

    function syncFilmstrip() : void {
        const index = selectedIndex()
        if (index < 0) {
            return
        }
        filmstrip.currentIndex = index
        filmstrip.positionViewAtIndex(index, ListView.Contain)
    }

    function playFrom(millis: int) : void {
        playback.playFrom(millis)
    }

    onSelectedAssetChanged: Qt.callLater(syncFilmstrip)
    onAssetsChanged: Qt.callLater(syncFilmstrip)

    AudioPlaybackWorkspace {
        id: playback
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: filmstripPanel.top
        asset: focusView.selectedAsset
        jobStats: focusView.jobStats
    }

    SoundSelectionToolbar {
        anchors.right: parent.right
        anchors.bottom: filmstripPanel.top
        anchors.rightMargin: 20
        anchors.bottomMargin: 14
        asset: focusView.selectedAsset
        z: 20
        onAffinityRequested: function(liked, rating) {
            focusView.affinityRequested(focusView.selectedAsset, liked, rating)
        }
    }

    Rectangle {
        id: filmstripPanel

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 108
        color: Theme.chrome
        border.width: 1
        border.color: Theme.border

        ListView {
            id: filmstrip

            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 14
            anchors.topMargin: 11
            anchors.bottomMargin: 11
            orientation: ListView.Horizontal
            spacing: 8
            clip: true
            cacheBuffer: 640
            model: focusView.assets
            focus: focusView.visible
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: Math.max(0, width * 0.38)
            preferredHighlightEnd: Math.max(0, width * 0.62)

            Keys.onPressed: function(event) {
                if (event.key !== Qt.Key_Left && event.key !== Qt.Key_Right) {
                    return
                }
                const direction = event.key === Qt.Key_Left ? -1 : 1
                const current = Math.max(0, focusView.selectedIndex())
                const next = Math.max(0, Math.min(count - 1, current + direction))
                if (next !== current && next < focusView.assets.length) {
                    focusView.assetSelected(focusView.assets[next])
                }
                event.accepted = true
            }

            delegate: SoundFilmstripItem {
                required property var modelData

                entry: modelData
                selected: focusView.selectedAsset !== null
                    && focusView.selectedAsset.id === modelData.id
                onActivated: focusView.assetSelected(modelData)
            }

            ScrollBar.horizontal: ScrollBar {}
        }

        Text {
            anchors.centerIn: parent
            visible: focusView.assets.length === 0
            text: qsTr("No sounds match these filters")
            color: Theme.textDisabled
            font.pixelSize: Theme.fontBody
        }
    }
}
