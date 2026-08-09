//! Audio Space is the Sound Wall projection controller. It owns collection
//! admission, search/sort, selected identity and browse-to-expanded routing;
//! sidebar, wall, inspector and playback remain independent UI owners.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Item {
    id: workspace

    property var selectedAsset: null
    property string selectedFilter: "all"
    property string searchText: ""
    property string sortMode: "date"
    property var allAssets: []
    property var smartAlbums: []
    property var jobStats: ({ pending: 0, running: 0, done: 0, failed: 0 })
    property int viewIndex: 0

    signal openLibraryRequested()

    function fileName(path: string) : string {
        const normalized = path.replace(/\\/g, "/")
        return normalized.substring(normalized.lastIndexOf("/") + 1)
    }

    function refreshAssets() : void {
        const selectedId = selectedAsset !== null ? selectedAsset.id : ""
        const assets = backend.listAssets()
        let reconciled = null
        for (const asset of assets) {
            if (asset.id === selectedId) {
                reconciled = asset
            }
        }
        allAssets = assets
        rebuildSmartAlbums()
        refilter()
        if (reconciled !== null) {
            selectedAsset = reconciled
        } else if (filteredAssets.count > 0) {
            selectedAsset = filteredAssets.get(0)
        } else {
            selectedAsset = null
        }
    }

    function rebuildSmartAlbums() : void {
        const collections = {}
        for (const asset of allAssets) {
            if (asset.sourceLocation.length > 0) {
                const key = "place:" + asset.sourceLocation
                collections[key] = {
                    key: key, label: asset.sourceLocation,
                    count: (collections[key] ? collections[key].count : 0) + 1,
                    aiSuggested: false
                }
            }
            if (asset.eventType.length > 0) {
                const key = "event:" + asset.eventType
                collections[key] = {
                    key: key, label: asset.eventType,
                    count: (collections[key] ? collections[key].count : 0) + 1,
                    aiSuggested: true
                }
            }
            if (asset.mood.length > 0) {
                const key = "mood:" + asset.mood
                collections[key] = {
                    key: key, label: asset.mood,
                    count: (collections[key] ? collections[key].count : 0) + 1,
                    aiSuggested: true
                }
            }
        }
        smartAlbums = Object.values(collections).sort((left, right) =>
            left.label.localeCompare(right.label))
    }

    function matchesCollection(asset: var) : bool {
        if (selectedFilter === "all") {
            return true
        }
        if (selectedFilter === "recent") {
            return asset.importedAtMillis >= Date.now() - 7 * 86400000
        }
        if (selectedFilter === "liked") {
            return asset.liked
        }
        if (selectedFilter === "five-star") {
            return asset.rating === 5
        }
        if (selectedFilter === "has-text") {
            return asset.textPreview.length > 0
        }
        if (selectedFilter === "missing") {
            return asset.pathStatus === "missing"
        }
        if (selectedFilter.startsWith("place:")) {
            return asset.sourceLocation === selectedFilter.substring(6)
        }
        if (selectedFilter.startsWith("event:")) {
            return asset.eventType === selectedFilter.substring(6)
        }
        if (selectedFilter.startsWith("mood:")) {
            return asset.mood === selectedFilter.substring(5)
        }
        return false
    }

    function matchingSearchIds() : var {
        const ids = {}
        const query = searchText.trim()
        if (query.length === 0) {
            return ids
        }
        for (const hit of backend.search(query)) {
            ids[hit.id] = true
        }
        return ids
    }

    function matchesSearch(asset: var, indexedIds: var) : bool {
        const query = searchText.trim().toLocaleLowerCase()
        if (query.length === 0) {
            return true
        }
        if (indexedIds[asset.id]) {
            return true
        }
        const haystack = [
            fileName(asset.path), asset.summary, asset.textPreview,
            asset.eventType, asset.mood, asset.sourceTitle, asset.sourceLocation,
            asset.keywords.join(" ")
        ].join(" ").toLocaleLowerCase()
        return haystack.includes(query)
    }

    function refilter() : void {
        const indexedIds = matchingSearchIds()
        const admitted = []
        for (const asset of allAssets) {
            if (matchesCollection(asset) && matchesSearch(asset, indexedIds)) {
                admitted.push(asset)
            }
        }
        admitted.sort((left, right) => {
            if (sortMode === "duration") {
                return right.durationMillis - left.durationMillis
            }
            if (sortMode === "rating") {
                return right.rating - left.rating
                    || right.importedAtMillis - left.importedAtMillis
            }
            const leftTime = left.recordedAtMillis > 0
                ? left.recordedAtMillis : left.importedAtMillis
            const rightTime = right.recordedAtMillis > 0
                ? right.recordedAtMillis : right.importedAtMillis
            return rightTime - leftTime
        })
        filteredAssets.clear()
        for (const asset of admitted) {
            filteredAssets.append(asset)
        }
        if (selectedAsset !== null
                && !admitted.some(asset => asset.id === selectedAsset.id)) {
            selectedAsset = admitted.length > 0 ? admitted[0] : null
        }
    }

    function collectionLabel() : string {
        const labels = {
            "all": qsTr("All sounds"),
            "recent": qsTr("Recently added"),
            "liked": qsTr("Liked"),
            "five-star": qsTr("5 stars"),
            "has-text": qsTr("With text"),
            "missing": qsTr("Missing originals")
        }
        if (labels[selectedFilter]) {
            return labels[selectedFilter]
        }
        for (const album of smartAlbums) {
            if (album.key === selectedFilter) {
                return album.label
            }
        }
        return qsTr("Sounds")
    }

    function selectFilter(key: string) : void {
        selectedFilter = key
        refilter()
        viewIndex = 0
    }

    function updateAffinity(asset: var, liked: bool, rating: int) : void {
        if (asset === null) {
            return
        }
        backend.setAssetAffinity(asset.id, liked, rating)
    }

    function openAsset(asset: var) : void {
        selectedAsset = asset
        viewIndex = 1
    }

    function selectSearchHit(hit: var) : void {
        for (const asset of allAssets) {
            if (asset.id === hit.id) {
                selectedAsset = asset
                viewIndex = 1
                Qt.callLater(() => expandedSound.playFrom(hit.startMillis))
                return
            }
        }
    }

    Connections {
        target: backend
        function onAssetsChanged() : void { workspace.refreshAssets() }
    }

    ListModel { id: filteredAssets }

    StackLayout {
        anchors.fill: parent
        currentIndex: workspace.viewIndex

        RowLayout {
            spacing: 0

            AudioLibrarySidebar {
                Layout.preferredWidth: 220
                Layout.minimumWidth: 205
                Layout.fillHeight: true
                assets: workspace.allAssets
                smartAlbums: workspace.smartAlbums
                selectedFilter: workspace.selectedFilter
                onFilterRequested: key => workspace.selectFilter(key)
                onManageLibraryRequested: workspace.openLibraryRequested()
            }

            SoundWall {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 560
                assets: filteredAssets
                selectedAsset: workspace.selectedAsset
                collectionLabel: workspace.collectionLabel()
                onAssetSelected: asset => workspace.selectedAsset = asset
                onAssetOpened: asset => workspace.openAsset(asset)
                onAffinityRequested: function(asset, liked, rating) {
                    workspace.updateAffinity(asset, liked, rating)
                }
                onSearchRequested: function(text) {
                    workspace.searchText = text
                    workspace.refilter()
                }
                onSortRequested: function(mode) {
                    workspace.sortMode = mode
                    workspace.refilter()
                }
            }

            SoundInspector {
                Layout.preferredWidth: 390
                Layout.minimumWidth: 350
                Layout.fillHeight: true
                asset: workspace.selectedAsset
                jobStats: workspace.jobStats
                onExpandRequested: asset => workspace.openAsset(asset)
                onAffinityRequested: function(asset, liked, rating) {
                    workspace.updateAffinity(asset, liked, rating)
                }
            }
        }

        ExpandedSoundWorkspace {
            id: expandedSound
            Layout.fillWidth: true
            Layout.fillHeight: true
            asset: workspace.selectedAsset
            jobStats: workspace.jobStats
            onCloseRequested: workspace.viewIndex = 0
        }
    }
}
