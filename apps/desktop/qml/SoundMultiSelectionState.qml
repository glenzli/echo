//! Authoritative modifier-aware selection for the current Audio Space result
//! projection. The inspector's primary sound is derived from this state, but
//! batch membership remains an explicit ordered identity set.

import QtQuick

QtObject {
    id: state

    required property var assets

    property var selectedIds: []
    property string primaryId: ""
    property int anchorIndex: -1

    readonly property int selectedCount: selectedIds.length

    function assetIndex(assetId: string): int {
        if (!assets || assets.length === undefined)
            return -1;
        for (let index = 0; index < assets.length; ++index) {
            if (assets[index].id === assetId)
                return index;
        }
        return -1;
    }

    function contains(assetId: string): bool {
        return selectedIds.indexOf(assetId) >= 0;
    }

    function orderedUnique(ids: var): var {
        const admitted = new Set(ids || []);
        const ordered = [];
        if (!assets || assets.length === undefined)
            return ordered;
        for (const asset of assets) {
            if (admitted.has(asset.id))
                ordered.push(asset.id);
        }
        return ordered;
    }

    function selectOnly(assetId: string): void {
        const index = assetIndex(assetId);
        if (index < 0) {
            selectedIds = [];
            primaryId = "";
            anchorIndex = -1;
            return;
        }
        selectedIds = [assetId];
        primaryId = assetId;
        anchorIndex = index;
    }

    function activate(assetId: string, modifiers: int): void {
        const index = assetIndex(assetId);
        if (index < 0)
            return;
        const shift = (modifiers & Qt.ShiftModifier) !== 0;
        const toggle = (modifiers & (Qt.MetaModifier | Qt.ControlModifier)) !== 0;
        let next = selectedIds.slice();

        if (shift && anchorIndex >= 0 && anchorIndex < assets.length) {
            const first = Math.min(anchorIndex, index);
            const last = Math.max(anchorIndex, index);
            const range = [];
            for (let cursor = first; cursor <= last; ++cursor)
                range.push(assets[cursor].id);
            next = toggle ? next.concat(range) : range;
        } else if (toggle) {
            const existing = next.indexOf(assetId);
            if (existing >= 0)
                next.splice(existing, 1);
            else
                next.push(assetId);
            anchorIndex = index;
        } else {
            next = [assetId];
            anchorIndex = index;
        }

        selectedIds = orderedUnique(next);
        if (selectedIds.indexOf(assetId) >= 0) {
            primaryId = assetId;
        } else if (selectedIds.length > 0) {
            primaryId = selectedIds[selectedIds.length - 1];
        } else {
            primaryId = "";
            anchorIndex = -1;
        }
    }

    function collapseToPrimary(): void {
        if (assetIndex(primaryId) >= 0)
            selectOnly(primaryId);
        else if (selectedIds.length > 0)
            selectOnly(selectedIds[0]);
        else {
            primaryId = "";
            anchorIndex = -1;
        }
    }

    function reconcile(): void {
        selectedIds = orderedUnique(selectedIds);
        if (selectedIds.indexOf(primaryId) < 0)
            primaryId = selectedIds.length > 0 ? selectedIds[0] : "";
        anchorIndex = assetIndex(primaryId);
    }

    onAssetsChanged: reconcile()
}
