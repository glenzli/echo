//! Audio Space is the Sound Wall projection controller. It owns collection
//! admission, search/sort, selected identity and browse presentation routing;
//! sidebar, wall, focus view, filter state and inspector remain independent.

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
    property string viewMode: "grid"
    property int preferredCardWidth: 236
    property bool likedOnly: false
    property int minimumRating: 0
    property bool speechOnly: false
    property var allAssets: []
    property var filteredAssets: []
    property var smartAlbums: []
    property var jobStats: ({ pending: 0, running: 0, done: 0, failed: 0 })

    readonly property int visibleAssetCount: filteredAssets.length
    readonly property string cardDensity: preferredCardWidth <= 205
        ? "overview" : preferredCardWidth <= 330 ? "browse" : "rich"

    signal openLibraryRequested()

    SoundFilterState {
        id: advancedFilterState
        assets: workspace.allAssets
    }

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

    function collectionTitle() : string {
        if (selectedFilter === "recent") return qsTr("Recently added")
        if (selectedFilter === "liked") return qsTr("Liked")
        if (selectedFilter === "five-star") return qsTr("5 stars")
        if (selectedFilter === "has-speech") return qsTr("With speech")
        if (selectedFilter === "missing") return qsTr("Missing originals")
        if (selectedFilter.startsWith("album:")) {
            const album = albumForKey(selectedFilter.substring(6))
            if (album !== null) return album.label
        }
        return qsTr("All sounds")
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
        if (selectedFilter === "has-speech") {
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
            fileName(asset.path), asset.soundCaption, asset.summary, asset.textPreview,
            asset.eventType, asset.mood, asset.sourceTitle, asset.sourceLocation,
            asset.keywords.join(" ")
        ].join(" ").toLocaleLowerCase()
        return haystack.includes(query)
    }

    function matchesFacets(asset: var) : bool {
        return (!likedOnly || asset.liked)
            && (minimumRating === 0 || asset.rating >= minimumRating)
            && (!speechOnly || asset.textPreview.length > 0)
            && advancedFilterState.matches(asset)
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

    function setSpeechOnly(enabled: bool) : void {
        speechOnly = enabled
        refilter()
    }

    function clearAllFilters() : void {
        likedOnly = false
        minimumRating = 0
        speechOnly = false
        advancedFilterState.clear()
    }

    function updateAffinity(asset: var, liked: bool, rating: int) : void {
        if (asset === null) {
            return
        }
        backend.setAssetAffinity(asset.id, liked, rating)
    }

    function openAsset(asset: var) : void {
        selectedAsset = asset
        viewMode = "focus"
    }

    function selectSearchHit(hit: var) : void {
        for (const asset of allAssets) {
            if (asset.id === hit.id) {
                selectedAsset = asset
                viewMode = "focus"
                Qt.callLater(() => soundFocus.playFrom(hit.startMillis))
                return
            }
        }
    }

    Connections {
        target: backend
        function onAssetsChanged() : void { workspace.refreshAssets() }
    }

    Connections {
        target: advancedFilterState
        function onFiltersChanged() : void { workspace.refilter() }
    }

    ColumnLayout {
        anchors.fill: parent
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

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumWidth: 560
                spacing: 0

                SoundPresentationToolbar {
                    Layout.fillWidth: true
                    collectionName: workspace.collectionTitle()
                    visibleCount: workspace.visibleAssetCount
                    viewMode: workspace.viewMode
                    cardWidth: workspace.preferredCardWidth
                    onViewModeRequested: mode => workspace.viewMode = mode
                    onCardWidthRequested: width => workspace.setPreferredCardWidth(width)
                }

                StackLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: workspace.viewMode === "grid" ? 0 : 1

                    SoundWall {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        assets: workspace.filteredAssets
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

                    SoundFocusView {
                        id: soundFocus

                        Layout.fillHeight: true
                        Layout.fillWidth: true
                        assets: workspace.filteredAssets
                        selectedAsset: workspace.selectedAsset
                        jobStats: workspace.jobStats
                        onAssetSelected: asset => workspace.selectedAsset = asset
                        onAffinityRequested: function(asset, liked, rating) {
                            workspace.updateAffinity(asset, liked, rating)
                        }
                    }
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
            speechOnly: workspace.speechOnly
            filterState: advancedFilterState
            onSortRequested: mode => workspace.setSortMode(mode)
            onLikedFilterRequested: enabled => workspace.setLikedOnly(enabled)
            onRatingFilterRequested: rating => workspace.setMinimumRating(rating)
            onSpeechFilterRequested: enabled => workspace.setSpeechOnly(enabled)
            onClearAllFiltersRequested: workspace.clearAllFilters()
        }
    }
}
