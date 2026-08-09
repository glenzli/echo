//! Audio Space owns collection filtering, transcript search, asset selection,
//! and live reconciliation when background imports complete.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Item {
    id: workspace

    property var selectedAsset: null
    property string selectedTag: "all"
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
        Qt.callLater(() => detailPane.playFrom(hit.startMillis))
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
        spacing: 14

        Rectangle {
            Layout.preferredWidth: 180
            Layout.fillHeight: true
            color: Theme.panel
            radius: Theme.panelRadius
            border.color: Theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 10

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 6
                    Layout.rightMargin: 6
                    spacing: 2

                    Text {
                        text: qsTr("Collections")
                        color: Theme.textPrimary
                        font.pixelSize: 14
                        font.bold: true
                    }

                    Text {
                        text: qsTr("Ways back into your sounds")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                    }
                }

                ListView {
                    id: collectionList

                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: collectionModel
                    spacing: 2

                    section.property: "group"
                    section.delegate: Text {
                        width: collectionList.width
                        height: 28
                        leftPadding: 6
                        text: section.toUpperCase()
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontMeta
                        font.letterSpacing: 0.6
                        verticalAlignment: Text.AlignBottom
                    }

                    delegate: Rectangle {
                        required property var modelData

                        width: collectionList.width
                        height: 34
                        radius: Theme.compactControlRadius
                        color: workspace.selectedTag === modelData.key
                            ? Theme.accentSurfaceQuiet : Theme.transparent

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

                EchoButton {
                    Layout.fillWidth: true
                    text: qsTr("Manage library")
                    ghost: true
                    onClicked: workspace.openLibraryRequested()
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: 280
            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Text {
                        text: qsTr("Audio Space")
                        color: Theme.textPrimary
                        font.pixelSize: 24
                        font.bold: true
                    }

                    Text {
                        text: assetModel.allAssets.length === 0
                            ? qsTr("A quiet place for the sounds you keep")
                            : qsTr("%1 recordings · originals stay untouched")
                                .arg(assetModel.allAssets.length)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontBody
                    }
                }

                Rectangle {
                    visible: workspace.jobStats.failed > 0
                    Layout.preferredWidth: failedText.implicitWidth + 16
                    Layout.preferredHeight: 26
                    radius: 13
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
                placeholderText: qsTr("Search words spoken in recordings…")
                onTextChanged: {
                    if (text.trim().length > 0) {
                        searchModel.refresh()
                    } else {
                        searchModel.clear()
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: Theme.panel
                radius: Theme.panelRadius
                border.color: Theme.border

                ListView {
                    id: assetList

                    anchors.fill: parent
                    anchors.margins: 8
                    visible: searchField.text.trim().length === 0 && assetModel.count > 0
                    spacing: 4
                    clip: true
                    model: assetModel

                    section.property: "timeBucket"
                    section.delegate: Text {
                        width: assetList.width
                        height: 30
                        leftPadding: 6
                        text: section
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSection
                        font.bold: true
                        verticalAlignment: Text.AlignVCenter
                    }

                    delegate: Rectangle {
                        required property var modelData

                        width: assetList.width
                        height: 72
                        radius: Theme.controlRadius
                        color: workspace.selectedAsset !== null
                            && workspace.selectedAsset.id === modelData.id
                            ? Theme.surfaceSelected : hovered.hovered
                                ? Theme.surfaceSubtle : Theme.transparent

                        HoverHandler { id: hovered }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: workspace.selectedAsset = modelData
                            onDoubleClicked: {
                                workspace.selectedAsset = modelData
                                Qt.callLater(() => detailPane.playFrom(0))
                            }
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            spacing: 11

                            Rectangle {
                                Layout.preferredWidth: 38
                                Layout.preferredHeight: 38
                                radius: 11
                                color: workspace.selectedAsset !== null
                                    && workspace.selectedAsset.id === modelData.id
                                    ? Theme.accentSurface : Theme.surfaceSubtle

                                EchoIcon {
                                    anchors.centerIn: parent
                                    source: "qrc:/EchoDesktop/icons/waveform.svg"
                                    size: 18
                                    color: workspace.selectedAsset !== null
                                        && workspace.selectedAsset.id === modelData.id
                                        ? Theme.accentSelectionText : Theme.textSecondary
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.summary.length > 0
                                        ? modelData.summary : workspace.fileName(modelData.path)
                                    color: Theme.textPrimary
                                    font.pixelSize: 13
                                    font.bold: modelData.summary.length > 0
                                    elide: Text.ElideRight
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 7

                                    Text {
                                        Layout.fillWidth: true
                                        text: modelData.summary.length > 0
                                            ? workspace.fileName(modelData.path)
                                            : modelData.path
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontMeta
                                        elide: Text.ElideMiddle
                                    }

                                    Text {
                                        text: workspace.formatDuration(modelData.durationMillis)
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontMeta
                                    }

                                    Rectangle {
                                        visible: modelData.pathStatus === "missing"
                                        Layout.preferredWidth: missingText.implicitWidth + 12
                                        Layout.preferredHeight: 18
                                        radius: 9
                                        color: Theme.warningSurface

                                        Text {
                                            id: missingText
                                            anchors.centerIn: parent
                                            text: qsTr("Missing")
                                            color: Theme.warningText
                                            font.pixelSize: Theme.fontMeta
                                        }
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
                    anchors.margins: 8
                    visible: searchField.text.trim().length > 0 && searchModel.count > 0
                    spacing: 4
                    clip: true
                    model: searchModel

                    delegate: Rectangle {
                        required property var modelData

                        width: searchList.width
                        height: 68
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
                            anchors.margins: 10
                            spacing: 4

                            Text {
                                Layout.fillWidth: true
                                text: modelData.snippet.replace(/ /g, "")
                                color: Theme.textPrimary
                                font.pixelSize: 13
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
                    width: Math.min(parent.width - 48, 300)
                    spacing: 10
                    visible: (searchField.text.trim().length === 0 && assetModel.count === 0)
                        || (searchField.text.trim().length > 0 && searchModel.count === 0)

                    Rectangle {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.preferredWidth: 54
                        Layout.preferredHeight: 54
                        radius: 17
                        color: Theme.surfaceSubtle

                        EchoIcon {
                            anchors.centerIn: parent
                            source: searchField.text.trim().length > 0
                                ? "qrc:/EchoDesktop/icons/tune.svg"
                                : "qrc:/EchoDesktop/icons/waveform.svg"
                            size: 24
                            color: Theme.textDisabled
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: searchField.text.trim().length > 0
                            ? qsTr("No spoken words found") : qsTr("Your Audio Space is empty")
                        color: Theme.textPrimary
                        font.pixelSize: 15
                        font.bold: true
                        horizontalAlignment: Text.AlignHCenter
                    }

                    Text {
                        Layout.fillWidth: true
                        text: searchField.text.trim().length > 0
                            ? qsTr("Try a shorter phrase or analyze a recording first.")
                            : qsTr("Add a folder to begin preserving and revisiting your recordings.")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontBody
                        wrapMode: Text.WordWrap
                        horizontalAlignment: Text.AlignHCenter
                    }

                    EchoButton {
                        visible: searchField.text.trim().length === 0
                        Layout.alignment: Qt.AlignHCenter
                        text: qsTr("Add recordings")
                        onClicked: workspace.openLibraryRequested()
                    }
                }
            }
        }

        AudioDetailPane {
            id: detailPane

            Layout.preferredWidth: 340
            Layout.fillHeight: true
            asset: workspace.selectedAsset
        }
    }
}
