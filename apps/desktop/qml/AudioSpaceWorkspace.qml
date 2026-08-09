//! Audio Space owns collection filtering, transcript search, asset selection,
//! and live reconciliation. Presentation is arranged as navigation, primary
//! playback workspace, and contextual inspector.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Item {
    id: workspace

    property var selectedAsset: null
    property string selectedTag: "all"
    property int totalAssetCount: 0
    property var jobStats: ({ pending: 0, running: 0, done: 0, failed: 0 })

    signal openLibraryRequested()

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function formatDuration(millis: int) : string {
        if (!millis || millis <= 0) {
            return qsTr("Unknown length")
        }
        const totalSeconds = Math.floor(millis / 1000)
        const minutes = Math.floor(totalSeconds / 60)
        const seconds = totalSeconds % 60
        return minutes + ":" + (seconds < 10 ? "0" : "") + seconds
    }

    function timeBucket(millis: double) : string {
        const date = new Date(millis)
        const now = new Date()
        if (date.toDateString() === now.toDateString()) {
            return qsTr("Today")
        }
        if (date > new Date(now.getTime() - 7 * 86400000)) {
            return qsTr("This week")
        }
        if (date > new Date(now.getTime() - 30 * 86400000)) {
            return qsTr("This month")
        }
        if (date.getFullYear() === now.getFullYear()) {
            return qsTr("This year")
        }
        return String(date.getFullYear())
    }

    function refreshAssets() : void {
        const selectedId = selectedAsset !== null ? selectedAsset.id : ""
        const assets = backend.listAssets()
        totalAssetCount = assets.length
        assetModel.allAssets = []
        let reconciled = null
        for (const asset of assets) {
            const moment = asset.recordedAtMillis > 0
                ? asset.recordedAtMillis : asset.importedAtMillis
            asset.timeBucket = timeBucket(moment)
            assetModel.allAssets.push(asset)
            if (asset.id === selectedId) {
                reconciled = asset
            }
        }
        if (reconciled === null && selectedId.length === 0 && assets.length > 0) {
            reconciled = assets[0]
        }
        selectedAsset = reconciled
        rebuildCollections(assets)
        assetModel.refilter()
        if (searchField.text.trim().length > 0) {
            searchModel.refresh()
        }
    }

    function rebuildCollections(assets: var) : void {
        collectionModel.clear()
        collectionModel.append({
            group: qsTr("Library"), key: "all",
            label: qsTr("All recordings"), count: assets.length
        })
        const events = {}
        const moods = {}
        for (const asset of assets) {
            if (asset.eventType.length > 0) {
                events[asset.eventType] = (events[asset.eventType] || 0) + 1
            }
            if (asset.mood.length > 0) {
                moods[asset.mood] = (moods[asset.mood] || 0) + 1
            }
        }
        for (const event of Object.keys(events).sort()) {
            collectionModel.append({
                group: qsTr("Sound memories"), key: event,
                label: event, count: events[event]
            })
        }
        for (const mood of Object.keys(moods).sort()) {
            collectionModel.append({
                group: qsTr("Mood"), key: mood,
                label: mood, count: moods[mood]
            })
        }
    }

    function selectSearchHit(hit: var) : void {
        let fullAsset = null
        for (const asset of assetModel.allAssets) {
            if (asset.id === hit.id) {
                fullAsset = asset
                break
            }
        }
        selectedAsset = fullAsset !== null ? fullAsset : hit
        Qt.callLater(() => previewPane.playFrom(hit.startMillis))
    }

    Connections {
        target: backend
        function onAssetsChanged() : void {
            workspace.refreshAssets()
        }
    }

    ListModel {
        id: collectionModel
    }

    ListModel {
        id: assetModel

        property var allAssets: []

        function refilter() : void {
            clear()
            for (const asset of allAssets) {
                if (workspace.selectedTag === "all"
                        || asset.eventType === workspace.selectedTag
                        || asset.mood === workspace.selectedTag) {
                    append(asset)
                }
            }
        }
    }

    ListModel {
        id: searchModel

        function refresh() : void {
            clear()
            const hits = backend.search(searchField.text)
            for (const hit of hits) {
                append(hit)
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 12

        Rectangle {
            Layout.preferredWidth: 276
            Layout.minimumWidth: 260
            Layout.maximumWidth: 300
            Layout.fillHeight: true
            color: Theme.panel
            radius: Theme.panelRadius
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 10

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 4
                    Layout.rightMargin: 4
                    spacing: 8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Text {
                            text: qsTr("Audio Space")
                            color: Theme.textPrimary
                            font.pixelSize: 17
                            font.bold: true
                        }

                        Text {
                            text: qsTr("%1 recordings").arg(workspace.totalAssetCount)
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                        }
                    }

                    Rectangle {
                        visible: workspace.jobStats.failed > 0
                        Layout.preferredWidth: failedText.implicitWidth + 14
                        Layout.preferredHeight: 22
                        radius: 11
                        color: Theme.warningSurface

                        Text {
                            id: failedText
                            anchors.centerIn: parent
                            text: qsTr("%1 failed").arg(workspace.jobStats.failed)
                            color: Theme.warningText
                            font.pixelSize: Theme.fontMeta
                        }
                    }
                }

                EchoTextField {
                    id: searchField

                    Layout.fillWidth: true
                    placeholderText: qsTr("Search spoken words…")
                    onTextChanged: {
                        if (text.trim().length > 0) {
                            searchModel.refresh()
                        } else {
                            searchModel.clear()
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 5
                    text: qsTr("COLLECTIONS")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.letterSpacing: 0.6
                }

                ListView {
                    id: collectionList

                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.min(176, Math.max(40, contentHeight))
                    clip: true
                    model: collectionModel
                    spacing: 2

                    section.property: "group"
                    section.delegate: Text {
                        width: collectionList.width
                        height: section === qsTr("Library") ? 0 : 24
                        leftPadding: 6
                        text: section === qsTr("Library") ? "" : section.toUpperCase()
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontMeta
                        verticalAlignment: Text.AlignBottom
                    }

                    delegate: Rectangle {
                        required property var modelData

                        width: collectionList.width
                        height: 32
                        radius: Theme.compactControlRadius
                        color: workspace.selectedTag === modelData.key
                            ? Theme.accentSurfaceQuiet : collectionHover.hovered
                                ? Theme.surfaceSubtle : Theme.transparent

                        HoverHandler { id: collectionHover }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                workspace.selectedTag = modelData.key
                                assetModel.refilter()
                            }
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 9
                            anchors.rightMargin: 8
                            spacing: 6

                            Text {
                                Layout.fillWidth: true
                                text: modelData.label
                                color: workspace.selectedTag === modelData.key
                                    ? Theme.accentSelectionText : Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                elide: Text.ElideRight
                            }

                            Text {
                                text: String(modelData.count)
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.border
                }

                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 5
                    Layout.rightMargin: 5

                    Text {
                        text: searchField.text.trim().length > 0
                            ? qsTr("SEARCH RESULTS") : qsTr("RECORDINGS")
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontMeta
                        font.letterSpacing: 0.6
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: String(searchField.text.trim().length > 0
                            ? searchModel.count : assetModel.count)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: Theme.transparent

                    ListView {
                        id: assetList

                        anchors.fill: parent
                        visible: searchField.text.trim().length === 0 && assetModel.count > 0
                        spacing: 3
                        clip: true
                        model: assetModel

                        section.property: "timeBucket"
                        section.delegate: Text {
                            width: assetList.width
                            height: 25
                            leftPadding: 6
                            text: section
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontMeta
                            font.bold: true
                            verticalAlignment: Text.AlignVCenter
                        }

                        delegate: Rectangle {
                            required property var modelData

                            width: assetList.width
                            height: 60
                            radius: Theme.controlRadius
                            color: workspace.selectedAsset !== null
                                && workspace.selectedAsset.id === modelData.id
                                ? Theme.surfaceSelected : assetHover.hovered
                                    ? Theme.surfaceSubtle : Theme.transparent

                            HoverHandler { id: assetHover }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: workspace.selectedAsset = modelData
                                onDoubleClicked: {
                                    workspace.selectedAsset = modelData
                                    Qt.callLater(() => previewPane.playFrom(0))
                                }
                            }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 8
                                anchors.rightMargin: 8
                                spacing: 9

                                Rectangle {
                                    Layout.preferredWidth: 34
                                    Layout.preferredHeight: 34
                                    radius: 10
                                    color: workspace.selectedAsset !== null
                                        && workspace.selectedAsset.id === modelData.id
                                        ? Theme.accentSurface : Theme.surfaceSubtle

                                    EchoIcon {
                                        anchors.centerIn: parent
                                        source: "qrc:/EchoDesktop/icons/waveform.svg"
                                        size: 16
                                        color: workspace.selectedAsset !== null
                                            && workspace.selectedAsset.id === modelData.id
                                            ? Theme.accentSelectionText : Theme.textSecondary
                                    }
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 3

                                    Text {
                                        Layout.fillWidth: true
                                        text: modelData.summary.length > 0
                                            ? modelData.summary : workspace.fileName(modelData.path)
                                        color: Theme.textPrimary
                                        font.pixelSize: Theme.fontBody
                                        font.bold: modelData.summary.length > 0
                                        elide: Text.ElideRight
                                    }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 5

                                        Text {
                                            Layout.fillWidth: true
                                            text: modelData.summary.length > 0
                                                ? workspace.fileName(modelData.path)
                                                : workspace.formatDuration(modelData.durationMillis)
                                            color: Theme.textSecondary
                                            font.pixelSize: Theme.fontMeta
                                            elide: Text.ElideRight
                                        }

                                        Text {
                                            visible: modelData.summary.length > 0
                                            text: workspace.formatDuration(modelData.durationMillis)
                                            color: Theme.textSecondary
                                            font.pixelSize: Theme.fontMeta
                                        }

                                        Text {
                                            visible: modelData.pathStatus === "missing"
                                            text: qsTr("Missing")
                                            color: Theme.warningText
                                            font.pixelSize: Theme.fontMeta
                                        }
                                    }
                                }
                            }
                        }

                        ScrollBar.vertical: ScrollBar {}
                    }

                    ListView {
                        id: searchList

                        anchors.fill: parent
                        visible: searchField.text.trim().length > 0 && searchModel.count > 0
                        spacing: 3
                        clip: true
                        model: searchModel

                        delegate: Rectangle {
                            required property var modelData

                            width: searchList.width
                            height: 58
                            radius: Theme.controlRadius
                            color: searchHover.hovered ? Theme.surfaceSubtle : Theme.transparent

                            HoverHandler { id: searchHover }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: workspace.selectSearchHit(modelData)
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: 8
                                spacing: 3

                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.snippet.replace(/ /g, "")
                                    color: Theme.textPrimary
                                    font.pixelSize: Theme.fontBody
                                    elide: Text.ElideRight
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: workspace.fileName(modelData.path)
                                        + " · " + workspace.formatDuration(modelData.startMillis)
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    elide: Text.ElideMiddle
                                }
                            }
                        }
                    }

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: Math.min(parent.width - 30, 220)
                        spacing: 8
                        visible: (searchField.text.trim().length === 0 && assetModel.count === 0)
                            || (searchField.text.trim().length > 0 && searchModel.count === 0)

                        Text {
                            Layout.fillWidth: true
                            text: searchField.text.trim().length > 0
                                ? qsTr("No spoken words found") : qsTr("Your Audio Space is empty")
                            color: Theme.textPrimary
                            font.pixelSize: 14
                            font.bold: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Text {
                            Layout.fillWidth: true
                            text: searchField.text.trim().length > 0
                                ? qsTr("Try a shorter phrase or analyze a recording first.")
                                : qsTr("Use the library button in the toolbar to add recordings.")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }

                        EchoButton {
                            visible: searchField.text.trim().length === 0
                            Layout.alignment: Qt.AlignHCenter
                            text: qsTr("Open library")
                            ghost: true
                            onClicked: workspace.openLibraryRequested()
                        }
                    }
                }
            }
        }

        AudioPlaybackWorkspace {
            id: previewPane

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 580
            asset: workspace.selectedAsset
        }

        AudioInspectorPane {
            Layout.preferredWidth: 284
            Layout.minimumWidth: 270
            Layout.maximumWidth: 310
            Layout.fillHeight: true
            asset: workspace.selectedAsset
        }
    }
}
