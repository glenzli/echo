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
    property int preferredCardWidth: 286
    property bool likedOnly: false
    property int minimumRating: 0
    property bool textOnly: false
    property var allAssets: []
    property var filteredAssets: []
    property var smartAlbums: []
    property var jobStats: ({ pending: 0, running: 0, done: 0, failed: 0 })
    property int viewIndex: 0

    readonly property int visibleAssetCount: filteredAssets.length
    readonly property string cardDensity: preferredCardWidth <= 260
        ? "overview" : preferredCardWidth <= 360 ? "browse" : "rich"

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
        smartAlbums = backend.listSmartAlbums()
        if (selectedFilter.startsWith("album:")
                && albumForKey(selectedFilter.substring(6)) === null) {
            selectedFilter = "all"
        }
        refilter()
        if (reconciled !== null) {
            selectedAsset = reconciled
        } else if (filteredAssets.length > 0) {
            selectedAsset = filteredAssets[0]
        } else {
            selectedAsset = null
        }
    }

    function albumForKey(key: string) : var {
        for (const album of smartAlbums) {
            if (album.key === key) {
                return album
            }
        }
        return null
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
        if (selectedFilter.startsWith("album:")) {
            const album = albumForKey(selectedFilter.substring(6))
            return album !== null && album.memberIds.includes(asset.id)
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

    function matchesFacets(asset: var) : bool {
        return (!likedOnly || asset.liked)
            && (minimumRating === 0 || asset.rating >= minimumRating)
            && (!textOnly || asset.textPreview.length > 0)
    }

    function refilter() : void {
        const indexedIds = matchingSearchIds()
        const admitted = []
        for (const asset of allAssets) {
            if (matchesCollection(asset) && matchesFacets(asset)
                    && matchesSearch(asset, indexedIds)) {
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
        filteredAssets = admitted
        if (selectedAsset !== null
                && !admitted.some(asset => asset.id === selectedAsset.id)) {
            selectedAsset = admitted.length > 0 ? admitted[0] : null
        }
    }

    function selectFilter(key: string) : void {
        selectedFilter = key
        refilter()
        viewIndex = 0
    }

    function setSearchText(text: string) : void {
        searchText = text
        refilter()
    }

    function setSortMode(mode: string) : void {
        sortMode = mode
        refilter()
    }

    function setPreferredCardWidth(value: real) : void {
        preferredCardWidth = Math.round(value)
    }

    function setLikedOnly(enabled: bool) : void {
        likedOnly = enabled
        refilter()
    }

    function setMinimumRating(rating: int) : void {
        minimumRating = Math.max(0, Math.min(5, rating))
        refilter()
    }

    function setTextOnly(enabled: bool) : void {
        textOnly = enabled
        refilter()
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

    StackLayout {
        anchors.fill: parent
        currentIndex: workspace.viewIndex

        ColumnLayout {
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
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
                    searchText: workspace.searchText
                    preferredCardWidth: workspace.preferredCardWidth
                    density: workspace.cardDensity
                    onAssetSelected: asset => workspace.selectedAsset = asset
                    onAssetOpened: asset => workspace.openAsset(asset)
                    onAffinityRequested: function(asset, liked, rating) {
                        workspace.updateAffinity(asset, liked, rating)
                    }
                }

                SoundInspector {
                    Layout.preferredWidth: 390
                    Layout.minimumWidth: 350
                    Layout.fillHeight: true
                    asset: workspace.selectedAsset
                    jobStats: workspace.jobStats
                    onAffinityRequested: function(asset, liked, rating) {
                        workspace.updateAffinity(asset, liked, rating)
                    }
                }
            }

            SoundWallBottomBar {
                Layout.fillWidth: true
                sortMode: workspace.sortMode
                likedOnly: workspace.likedOnly
                minimumRating: workspace.minimumRating
                textOnly: workspace.textOnly
                visibleCount: workspace.visibleAssetCount
                totalCount: workspace.allAssets.length
                cardWidth: workspace.preferredCardWidth
                onSortRequested: mode => workspace.setSortMode(mode)
                onLikedFilterRequested: enabled => workspace.setLikedOnly(enabled)
                onRatingFilterRequested: rating => workspace.setMinimumRating(rating)
                onTextFilterRequested: enabled => workspace.setTextOnly(enabled)
                onCardWidthRequested: width => workspace.setPreferredCardWidth(width)
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
