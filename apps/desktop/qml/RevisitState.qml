//! Revisit's bounded desktop projection. Catalog owns admission and ordering;
//! this owner resolves returned identities against the current Library
//! snapshot and decorates user albums with their cover assets.

import QtQuick

QtObject {
    id: state

    required property var catalogBackend
    property var assets: []
    property var userAlbums: []

    property var continueListening: []
    property var recentlyListened: []
    property var onThisDay: []
    property var recentlyAdded: []
    property var albumCards: []
    property int revision: 0

    readonly property int visibleAssetCount: uniqueVisibleAssetCount()

    function uniqueVisibleAssetCount(): int {
        const identities = {};
        for (const section of [continueListening, recentlyListened, onThisDay, recentlyAdded]) {
            for (const asset of section)
                identities[asset.id] = true;
        }
        return Object.keys(identities).length;
    }

    function resolve(ids: var, byId: var): var {
        const resolved = [];
        for (const id of ids || []) {
            if (byId[id] !== undefined)
                resolved.push(byId[id]);
        }
        return resolved;
    }

    function refresh(): void {
        const byId = {};
        for (const asset of assets)
            byId[asset.id] = asset;

        const snapshot = catalogBackend.revisitSnapshot();
        continueListening = resolve(snapshot.continueListeningAssetIds || [], byId);
        recentlyListened = resolve(snapshot.recentlyListenedAssetIds || [], byId);
        onThisDay = resolve(snapshot.onThisDayAssetIds || [], byId);
        recentlyAdded = resolve(snapshot.recentlyAddedAssetIds || [], byId);

        const projectedAlbums = [];
        for (let index = 0; index < Math.min(userAlbums.length, 8); ++index) {
            const album = userAlbums[index];
            projectedAlbums.push(Object.assign({}, album, {
                coverAsset: byId[album.coverAssetId] || null
            }));
        }
        albumCards = projectedAlbums;
        revision += 1;
    }
}
