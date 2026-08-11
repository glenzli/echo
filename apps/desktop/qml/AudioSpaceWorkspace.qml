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
    property int preferredCardWidth: 224
    property bool likedOnly: false
    property int minimumRating: 0
    property bool speechOnly: false
    property string analysisFilter: "all"
    property var allAssets: []
    property var filteredAssets: []
    property var processingRecipes: []
    property int processingRecipeModelRevision: 0
    property var processingHistory: []
    property int processingHistoryModelRevision: 0
    property string processingRecipeNotice: ""
    property string lastProcessingRecipeBatchId: ""
    property var jobStats: ({
            pending: 0,
            running: 0,
            done: 0,
            failed: 0
        })

    readonly property int visibleAssetCount: filteredAssets.length
    readonly property string cardDensity: preferredCardWidth <= 205 ? "overview" : preferredCardWidth <= 330 ? "browse" : "rich"
    readonly property int incompleteAnalysisCount: allAssets.filter(asset => asset.analysisState !== "done").length
    readonly property int failedAnalysisCount: allAssets.filter(asset => asset.analysisState === "failed" || asset.analysisState === "cancelled").length
    readonly property int manualAnalysisCount: allAssets.filter(asset => asset.analysisRecovery === "manual").length

    signal openLibraryRequested

    SoundFilterState {
        id: advancedFilterState
        assets: workspace.allAssets
    }

    SoundAlbumState {
        id: albumState
        catalogBackend: backend
    }

    SoundMultiSelectionState {
        id: soundSelection
        assets: workspace.filteredAssets

        onPrimaryIdChanged: {
            const primary = workspace.assetForId(primaryId);
            if (primary !== null)
                workspace.selectedAsset = primary;
        }
    }

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function normalizedSearchText(): string {
        return searchText.trim().replace(/\s+/g, " ");
    }

    function refreshAssets(): void {
        const selectedId = selectedAsset !== null ? selectedAsset.id : "";
        const assets = backend.listAssets();
        allAssets = assets;
        albumState.refresh();
        reconcileAlbumFilter();
        refilter();
        soundSelection.reconcile();
        if (soundSelection.selectedCount > 0) {
            selectedAsset = assetForId(soundSelection.primaryId);
        } else {
            const reconciled = assetForId(selectedId);
            selectAssetOnly(reconciled !== null ? reconciled : filteredAssets.length > 0 ? filteredAssets[0] : null);
        }
    }

    function refreshAnalysisStatuses(): void {
        const projected = backend.analysisStatuses();
        if (projected.length === 0 || allAssets.length === 0)
            return;
        const byId = {};
        for (const status of projected)
            byId[status.assetId] = status;
        const selectedId = selectedAsset !== null ? selectedAsset.id : "";
        allAssets = allAssets.map(asset => {
            const status = byId[asset.id];
            if (status === undefined)
                return asset;
            return Object.assign({}, asset, {
                analysisStage: status.stage,
                analysisState: status.state,
                analysisRecovery: status.recovery,
                analysisErrorCode: status.errorCode,
                analysisProgress: status.progress,
                analysisAttempts: status.attempts
            });
        });
        refilter();
        const refreshed = assetForId(selectedId);
        if (refreshed !== null)
            selectedAsset = refreshed;
    }

    function assetForId(assetId: string): var {
        for (const asset of filteredAssets) {
            if (asset.id === assetId)
                return asset;
        }
        return null;
    }

    function selectAssetOnly(asset: var): void {
        selectedAsset = asset;
        soundSelection.selectOnly(asset !== null ? asset.id : "");
    }

    function activateAsset(asset: var, modifiers: int): void {
        if (asset === null)
            return;
        soundSelection.activate(asset.id, modifiers);
        const primary = assetForId(soundSelection.primaryId);
        if (primary !== null)
            selectedAsset = primary;
    }

    function refreshProcessingRecipes(): void {
        processingRecipes = backend.listProcessingRecipes();
        processingRecipeModelRevision += 1;
    }

    function presentProcessingRecipesFor(targetIds: var, targetLabel: string): void {
        refreshProcessingRecipes();
        processingRecipeApplyDialog.recipeModel = processingRecipes;
        processingRecipeApplyDialog.modelRevision = processingRecipeModelRevision;
        processingRecipeApplyDialog.selectedRecipeId = processingRecipes.length > 0 ? processingRecipes[0].id : "";
        processingRecipeApplyDialog.mergeMode = "merge";
        processingRecipeApplyDialog.targetIds = targetIds;
        processingRecipeApplyDialog.targetLabel = targetLabel;
        processingRecipeApplyDialog.present();
    }

    function presentProcessingRecipes(): void {
        presentProcessingRecipesFor(filteredAssets.map(asset => asset.id), qsTr("Current results · %1 sounds").arg(filteredAssets.length));
    }

    function presentSelectedProcessingRecipes(): void {
        presentProcessingRecipesFor(soundSelection.selectedIds.slice(), qsTr("Selected sounds · %1").arg(soundSelection.selectedCount));
    }

    function presentProcessingRecipeManager(): void {
        refreshProcessingRecipeManager("");
        processingRecipeManagerDialog.sourceAssetId = selectedAsset !== null ? selectedAsset.id : "";
        processingRecipeManagerDialog.sourceLabel = selectedAsset !== null ? fileName(selectedAsset.path) : "";
        processingRecipeManagerDialog.canUpdateFromSource = selectedAsset !== null && Number(selectedAsset.adjustmentRevision || 0) > 0;
        processingRecipeManagerDialog.present();
    }

    function refreshProcessingRecipeManager(preferredId: string): void {
        refreshProcessingRecipes();
        processingRecipeManagerDialog.recipeModel = processingRecipes;
        processingRecipeManagerDialog.modelRevision = processingRecipeModelRevision;
        const stillExists = processingRecipes.some(recipe => recipe.id === preferredId);
        processingRecipeManagerDialog.selectedRecipeId = stillExists ? preferredId : processingRecipes.length > 0 ? processingRecipes[0].id : "";
    }

    function refreshProcessingHistory(): void {
        processingHistory = backend.listProcessingRecipeHistory();
        processingHistoryModelRevision += 1;
    }

    function presentProcessingHistory(): void {
        refreshProcessingHistory();
        processingHistoryDialog.present();
    }

    function applyProcessingRecipe(recipeId: string, mergeMode: string, targetIds: var): void {
        const receipt = backend.applyProcessingRecipe(recipeId, targetIds, mergeMode);
        if (!receipt || !receipt.batchId) {
            processingRecipeNotice = qsTr("The processing recipe could not be applied.");
            lastProcessingRecipeBatchId = "";
        } else {
            processingRecipeNotice = qsTr("%1 updated · %2 unchanged · %3 failed").arg(receipt.updatedCount).arg(receipt.unchangedCount).arg(receipt.failedCount);
            lastProcessingRecipeBatchId = receipt.batchId;
            refreshProcessingHistory();
        }
        processingRecipeNoticePopup.open();
        processingRecipeNoticeTimer.restart();
    }

    function revertLastProcessingRecipeApplication(): void {
        if (lastProcessingRecipeBatchId.length === 0)
            return;
        revertProcessingRecipeBatch(lastProcessingRecipeBatchId);
    }

    function revertProcessingRecipeBatch(batchId: string): void {
        if (batchId.length === 0)
            return;
        const receipt = backend.revertProcessingRecipeApplication(batchId);
        if (!receipt || !receipt.revertId) {
            processingRecipeNotice = qsTr("The processing recipe application could not be undone.");
        } else {
            processingRecipeNotice = qsTr("%1 restored · %2 conflicts · %3 failed").arg(receipt.restoredCount).arg(receipt.conflictCount).arg(receipt.failedCount);
            if (lastProcessingRecipeBatchId === batchId)
                lastProcessingRecipeBatchId = "";
        }
        refreshProcessingHistory();
        processingRecipeNoticePopup.open();
        processingRecipeNoticeTimer.restart();
    }

    function reconcileAlbumFilter(): void {
        if (selectedFilter.startsWith("user-album:")) {
            const albumId = Number(selectedFilter.substring(11));
            if (albumState.userAlbum(albumId) === null)
                selectedFilter = "all";
        } else if (selectedFilter.startsWith("suggested-album:")) {
            const key = selectedFilter.substring(16);
            if (albumState.suggestedAlbum(key) === null)
                selectedFilter = "all";
        }
    }

    function collectionTitle(): string {
        if (selectedFilter === "recent")
            return qsTr("Recently added");
        if (selectedFilter === "liked")
            return qsTr("Liked");
        if (selectedFilter === "five-star")
            return qsTr("5 stars");
        if (selectedFilter === "has-speech")
            return qsTr("With speech");
        if (selectedFilter === "missing")
            return qsTr("Missing originals");
        if (selectedFilter.startsWith("user-album:")) {
            const album = albumState.userAlbum(Number(selectedFilter.substring(11)));
            if (album !== null)
                return album.name;
        }
        if (selectedFilter.startsWith("suggested-album:")) {
            const album = albumState.suggestedAlbum(selectedFilter.substring(16));
            if (album !== null)
                return album.label;
        }
        return qsTr("All sounds");
    }

    function matchesCollection(asset: var): bool {
        if (selectedFilter === "all") {
            return true;
        }
        if (selectedFilter === "recent") {
            return asset.importedAtMillis >= Date.now() - 7 * 86400000;
        }
        if (selectedFilter === "liked") {
            return asset.liked;
        }
        if (selectedFilter === "five-star") {
            return asset.rating === 5;
        }
        if (selectedFilter === "has-speech") {
            return asset.textPreview.length > 0;
        }
        if (selectedFilter === "missing") {
            return asset.pathStatus === "missing";
        }
        if (selectedFilter.startsWith("user-album:")) {
            const album = albumState.userAlbum(Number(selectedFilter.substring(11)));
            return album !== null && album.memberIds.includes(asset.id);
        }
        if (selectedFilter.startsWith("suggested-album:")) {
            const album = albumState.suggestedAlbum(selectedFilter.substring(16));
            return album !== null && album.memberIds.includes(asset.id);
        }
        return false;
    }

    function matchingSearchScores(): var {
        const ids = {};
        const query = normalizedSearchText();
        if (query.length === 0) {
            return ids;
        }
        for (const hit of backend.search(query)) {
            ids[hit.id] = 2;
        }
        if (semanticSearch.resultsQuery === query) {
            for (const hit of semanticSearch.results) {
                ids[hit.id] = Math.max(ids[hit.id] || 0, hit.score);
            }
        }
        return ids;
    }

    function matchesSearch(asset: var, indexedScores: var): bool {
        const query = searchText.trim().toLocaleLowerCase();
        if (query.length === 0) {
            return true;
        }
        if (indexedScores[asset.id] !== undefined) {
            return true;
        }
        const haystack = [fileName(asset.path), asset.soundCaption, asset.summary, asset.textPreview, asset.eventType, asset.mood, asset.language, asset.sourceTitle, asset.sourceLocation, asset.keywords.join(" ")].join(" ").toLocaleLowerCase();
        return haystack.includes(query);
    }

    function searchRank(asset: var, indexedScores: var): real {
        const query = normalizedSearchText().toLocaleLowerCase();
        if (query.length === 0)
            return 0;
        const haystack = [fileName(asset.path), asset.soundCaption, asset.summary, asset.textPreview, asset.eventType, asset.mood, asset.language, asset.sourceTitle, asset.sourceLocation, asset.keywords.join(" ")].join(" ").toLocaleLowerCase();
        if (haystack.includes(query))
            return 3;
        return indexedScores[asset.id] === undefined ? 0 : indexedScores[asset.id];
    }

    function matchesFacets(asset: var): bool {
        const analysisMatches = analysisFilter === "all" || analysisFilter === "incomplete" && asset.analysisState !== "done" || analysisFilter === "failed" && (asset.analysisState === "failed" || asset.analysisState === "cancelled");
        return analysisMatches && (!likedOnly || asset.liked) && (minimumRating === 0 || asset.rating >= minimumRating) && (!speechOnly || asset.textPreview.length > 0) && advancedFilterState.matches(asset);
    }

    function refilter(): void {
        const indexedScores = matchingSearchScores();
        const admitted = [];
        for (const asset of allAssets) {
            if (matchesCollection(asset) && matchesFacets(asset) && matchesSearch(asset, indexedScores)) {
                admitted.push(asset);
            }
        }
        admitted.sort((left, right) => {
            if (normalizedSearchText().length > 0) {
                const relevance = searchRank(right, indexedScores) - searchRank(left, indexedScores);
                if (Math.abs(relevance) > 0.000001)
                    return relevance;
            }
            if (sortMode === "duration") {
                return right.durationMillis - left.durationMillis;
            }
            if (sortMode === "rating") {
                return right.rating - left.rating || right.importedAtMillis - left.importedAtMillis;
            }
            const leftTime = left.recordedAtMillis > 0 ? left.recordedAtMillis : left.importedAtMillis;
            const rightTime = right.recordedAtMillis > 0 ? right.recordedAtMillis : right.importedAtMillis;
            return rightTime - leftTime;
        });
        filteredAssets = admitted;
        soundSelection.reconcile();
        if (soundSelection.selectedCount > 0) {
            selectedAsset = assetForId(soundSelection.primaryId);
        } else if (selectedAsset !== null && admitted.some(asset => asset.id === selectedAsset.id)) {
            soundSelection.selectOnly(selectedAsset.id);
        } else {
            selectAssetOnly(admitted.length > 0 ? admitted[0] : null);
        }
    }

    function selectFilter(key: string): void {
        selectedFilter = key;
        refilter();
    }

    function setSearchText(text: string): void {
        searchText = text;
        refilter();
        semanticSearchTimer.restart();
    }

    function setSortMode(mode: string): void {
        sortMode = mode;
        refilter();
    }

    function setPreferredCardWidth(value: real): void {
        preferredCardWidth = Math.round(value);
    }

    function setLikedOnly(enabled: bool): void {
        likedOnly = enabled;
        refilter();
    }

    function setMinimumRating(rating: int): void {
        minimumRating = Math.max(0, Math.min(5, rating));
        refilter();
    }

    function setSpeechOnly(enabled: bool): void {
        speechOnly = enabled;
        refilter();
    }

    function setAnalysisFilter(filter: string): void {
        analysisFilter = analysisFilter === filter ? "all" : filter;
        refilter();
    }

    function clearAllFilters(): void {
        likedOnly = false;
        minimumRating = 0;
        speechOnly = false;
        analysisFilter = "all";
        advancedFilterState.clear();
    }

    function retryAnalysis(asset: var): void {
        if (asset === null)
            return;
        const retried = backend.retryAnalysis(asset.id);
        processingRecipeNotice = retried ? qsTr("Analysis restarted. Completed stages were kept.") : qsTr("This analysis stage could not be restarted.");
        processingRecipeNoticePopup.open();
        processingRecipeNoticeTimer.restart();
        Qt.callLater(refreshAnalysisStatuses);
    }

    function retryFailedAnalysis(): void {
        const retried = backend.retryFailedAnalysis();
        processingRecipeNotice = retried > 0 ? qsTr("%1 analysis job(s) restarted. Completed stages were kept.").arg(retried) : qsTr("No manually recoverable analysis was found.");
        processingRecipeNoticePopup.open();
        processingRecipeNoticeTimer.restart();
        Qt.callLater(refreshAnalysisStatuses);
    }

    function updateAffinity(asset: var, liked: bool, rating: int): void {
        if (asset === null) {
            return;
        }
        backend.setAssetAffinity(asset.id, liked, rating);
    }

    function createAlbum(name: string, memberIds: var): void {
        const albumId = albumState.createAlbum(name, memberIds);
        if (albumId >= 0) {
            selectedFilter = "user-album:" + albumId;
            refilter();
        }
    }

    function renameAlbum(albumId: var, name: string): void {
        albumState.renameAlbum(albumId, name);
    }

    function deleteAlbum(albumId: var): void {
        if (albumState.deleteAlbum(albumId) && selectedFilter === "user-album:" + albumId) {
            selectedFilter = "all";
            refilter();
        }
    }

    function saveSuggestedAlbum(album: var, name: string): void {
        createAlbum(name, album.memberIds);
    }

    function setAlbumMembership(asset: var, album: var, included: bool): void {
        if (asset === null)
            return;
        albumState.setMembership(album.id, asset.id, included);
    }

    function openAsset(asset: var): void {
        selectAssetOnly(asset);
        viewMode = "focus";
    }

    function debugOpenNewAlbumDialog(): void {
        librarySidebar.beginCreate([]);
    }

    function debugCreateAlbum(name: string): void {
        const members = selectedAsset !== null ? [selectedAsset.id] : [];
        createAlbum(name, members);
    }

    function debugOpenBatchDialog(): void {
        batchExportDialog.present();
    }

    function debugBatchExport(destination: url, format: string): void {
        batchExportDialog.destination = destination;
        batchExportDialog.selectedFormat = format;
        batchExporter.start(filteredAssets, destination, format);
    }

    function selectSearchHit(hit: var): void {
        for (const asset of allAssets) {
            if (asset.id === hit.id) {
                selectAssetOnly(asset);
                viewMode = "focus";
                Qt.callLater(() => soundFocus.playFrom(hit.startMillis));
                return;
            }
        }
    }

    Connections {
        target: backend
        function onAssetsChanged(): void {
            workspace.refreshAssets();
        }
        function onJobsChanged(): void {
            workspace.refreshAnalysisStatuses();
        }
        function onProcessingRecipesChanged(): void {
            workspace.refreshProcessingRecipes();
        }
    }

    Connections {
        target: semanticSearch
        function onResultsChanged(): void {
            if (semanticSearch.resultsQuery === workspace.normalizedSearchText()) {
                workspace.refilter();
            }
        }
    }

    Timer {
        id: semanticSearchTimer
        interval: 320
        repeat: false
        onTriggered: {
            const query = workspace.normalizedSearchText();
            if (query.length === 0)
                semanticSearch.clear();
            else
                semanticSearch.request(query);
        }
    }

    Connections {
        target: advancedFilterState
        function onFiltersChanged(): void {
            workspace.refilter();
        }
    }

    Connections {
        target: albumState
        function onAlbumsRefreshed(): void {
            workspace.reconcileAlbumFilter();
            workspace.refilter();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            AudioLibrarySidebar {
                id: librarySidebar

                Layout.preferredWidth: 228
                Layout.minimumWidth: 212
                Layout.fillHeight: true
                assets: workspace.allAssets
                userAlbums: albumState.userAlbums
                suggestedAlbums: albumState.suggestedAlbums
                selectedFilter: workspace.selectedFilter
                albumError: albumState.errorMessage
                onFilterRequested: key => workspace.selectFilter(key)
                onManageLibraryRequested: workspace.openLibraryRequested()
                onCreateAlbumRequested: function (name, memberIds) {
                    workspace.createAlbum(name, memberIds);
                }
                onRenameAlbumRequested: function (albumId, name) {
                    workspace.renameAlbum(albumId, name);
                }
                onDeleteAlbumRequested: albumId => workspace.deleteAlbum(albumId)
                onSaveSuggestedAlbumRequested: function (album, name) {
                    workspace.saveSuggestedAlbum(album, name);
                }
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
                    searchText: workspace.searchText
                    semanticSearching: semanticSearch.running
                    onSearchRequested: text => workspace.setSearchText(text)
                    onViewModeRequested: mode => workspace.viewMode = mode
                    onCardWidthRequested: width => workspace.setPreferredCardWidth(width)
                    onBatchExportRequested: batchExportDialog.present()
                    onProcessingRecipesRequested: workspace.presentProcessingRecipes()
                    onProcessingRecipeManagementRequested: workspace.presentProcessingRecipeManager()
                    onProcessingHistoryRequested: workspace.presentProcessingHistory()
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
                        selectedAssetIds: soundSelection.selectedIds
                        searchText: workspace.searchText
                        preferredCardWidth: workspace.preferredCardWidth
                        density: workspace.cardDensity
                        userAlbums: albumState.userAlbums
                        onAssetSelectionRequested: function (asset, modifiers) {
                            workspace.activateAsset(asset, modifiers);
                        }
                        onAssetOpened: asset => workspace.openAsset(asset)
                        onProcessingRecipeRequested: workspace.presentSelectedProcessingRecipes()
                        onSelectionClearRequested: soundSelection.collapseToPrimary()
                        onAffinityRequested: function (asset, liked, rating) {
                            workspace.updateAffinity(asset, liked, rating);
                        }
                        onAlbumMembershipRequested: function (asset, album, included) {
                            workspace.setAlbumMembership(asset, album, included);
                        }
                        onCreateAlbumRequested: librarySidebar.beginCreate(workspace.selectedAsset !== null ? [workspace.selectedAsset.id] : [])
                    }

                    SoundFocusView {
                        id: soundFocus

                        Layout.fillHeight: true
                        Layout.fillWidth: true
                        assets: workspace.filteredAssets
                        selectedAsset: workspace.selectedAsset
                        jobStats: workspace.jobStats
                        userAlbums: albumState.userAlbums
                        onAssetSelected: asset => workspace.selectAssetOnly(asset)
                        onAffinityRequested: function (asset, liked, rating) {
                            workspace.updateAffinity(asset, liked, rating);
                        }
                        onAlbumMembershipRequested: function (asset, album, included) {
                            workspace.setAlbumMembership(asset, album, included);
                        }
                        onCreateAlbumRequested: librarySidebar.beginCreate(workspace.selectedAsset !== null ? [workspace.selectedAsset.id] : [])
                    }
                }
            }

            SoundInspector {
                Layout.preferredWidth: 390
                Layout.minimumWidth: 350
                Layout.fillHeight: true
                asset: workspace.selectedAsset
                jobStats: workspace.jobStats
                onAffinityRequested: function (asset, liked, rating) {
                    workspace.updateAffinity(asset, liked, rating);
                }
                onRetryAnalysisRequested: asset => workspace.retryAnalysis(asset)
            }
        }

        SoundWallBottomBar {
            Layout.fillWidth: true
            sortMode: workspace.sortMode
            likedOnly: workspace.likedOnly
            minimumRating: workspace.minimumRating
            speechOnly: workspace.speechOnly
            analysisFilter: workspace.analysisFilter
            incompleteAnalysisCount: workspace.incompleteAnalysisCount
            failedAnalysisCount: workspace.failedAnalysisCount
            manualAnalysisCount: workspace.manualAnalysisCount
            filterState: advancedFilterState
            onSortRequested: mode => workspace.setSortMode(mode)
            onLikedFilterRequested: enabled => workspace.setLikedOnly(enabled)
            onRatingFilterRequested: rating => workspace.setMinimumRating(rating)
            onSpeechFilterRequested: enabled => workspace.setSpeechOnly(enabled)
            onAnalysisFilterRequested: filter => workspace.setAnalysisFilter(filter)
            onRetryFailedAnalysisRequested: workspace.retryFailedAnalysis()
            onClearAllFiltersRequested: workspace.clearAllFilters()
        }
    }

    BatchExportDialog {
        id: batchExportDialog
        assets: workspace.filteredAssets
        exporter: batchExporter
        collectionName: workspace.collectionTitle()
    }

    ProcessingRecipeApplyDialog {
        id: processingRecipeApplyDialog

        onRecipeSelected: recipeId => selectedRecipeId = recipeId
        onMergeModeSelected: mode => mergeMode = mode
        onApplyRequested: function (recipeId, mergeMode, targetIds) {
            workspace.applyProcessingRecipe(recipeId, mergeMode, targetIds);
        }
    }

    ProcessingRecipeManagerDialog {
        id: processingRecipeManagerDialog

        onRecipeSelected: recipeId => selectedRecipeId = recipeId
        onRenameRequested: function (recipeId, name) {
            if (backend.renameProcessingRecipe(recipeId, name)) {
                workspace.refreshProcessingRecipeManager(recipeId);
                workspace.processingRecipeNotice = qsTr("Processing recipe renamed.");
            } else {
                workspace.processingRecipeNotice = qsTr("The processing recipe could not be renamed.");
            }
            processingRecipeNoticePopup.open();
            processingRecipeNoticeTimer.restart();
        }
        onUpdateRequested: function (recipeId, componentIds) {
            const revision = backend.updateProcessingRecipe(recipeId, sourceAssetId, componentIds);
            if (revision > 0) {
                workspace.refreshProcessingRecipeManager(recipeId);
                workspace.processingRecipeNotice = qsTr("Processing recipe version %1 added.").arg(revision);
            } else {
                workspace.processingRecipeNotice = qsTr("The processing recipe could not be updated.");
            }
            processingRecipeNoticePopup.open();
            processingRecipeNoticeTimer.restart();
        }
        onArchiveRequested: function (recipeId) {
            if (backend.archiveProcessingRecipe(recipeId)) {
                workspace.refreshProcessingRecipeManager("");
                workspace.processingRecipeNotice = qsTr("Processing recipe archived.");
            } else {
                workspace.processingRecipeNotice = qsTr("The processing recipe could not be archived.");
            }
            processingRecipeNoticePopup.open();
            processingRecipeNoticeTimer.restart();
        }
    }

    ProcessingHistoryDialog {
        id: processingHistoryDialog

        historyModel: workspace.processingHistory
        modelRevision: workspace.processingHistoryModelRevision
        onRevertRequested: batchId => workspace.revertProcessingRecipeBatch(batchId)
    }

    Popup {
        id: processingRecipeNoticePopup

        parent: Overlay.overlay
        x: Math.round((parent.width - width) / 2)
        y: 18
        implicitWidth: Math.min(560, noticeRow.implicitWidth + 28)
        implicitHeight: noticeRow.implicitHeight + 20
        padding: 0
        closePolicy: Popup.NoAutoClose

        background: Rectangle {
            radius: Theme.controlRadius
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: RowLayout {
            id: noticeRow

            spacing: 10

            Text {
                Layout.fillWidth: true
                text: workspace.processingRecipeNotice
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }

            EchoButton {
                visible: workspace.lastProcessingRecipeBatchId.length > 0
                text: qsTr("Undo batch")
                ghost: true
                onClicked: workspace.revertLastProcessingRecipeApplication()
            }
        }
    }

    Timer {
        id: processingRecipeNoticeTimer
        interval: workspace.lastProcessingRecipeBatchId.length > 0 ? 6000 : 3200
        onTriggered: processingRecipeNoticePopup.close()
    }
}
